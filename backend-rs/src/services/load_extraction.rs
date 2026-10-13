//! Screenshot → load fields via the Claude API. There's no official Rust SDK, so this is a
//! raw `POST /v1/messages`. Structured outputs (`output_config.format`) guarantee the reply
//! is JSON matching the schema, so there's no fence-stripping or "respond with ONLY JSON".

use base64::Engine;
use chrono::NaiveDate;
use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{error::AppError, models::money, state::AppState};

const MODEL: &str = "claude-opus-5-5";
const ANTHROPIC_VERSION: &str = "2023-06-01";
/// Gates `"fallbacks": "default"`: if a safety classifier declines, the API retries on
/// Anthropic's recommended fallback model inside the same call.
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";

const PROMPT: &str = "Extract the load date, store name/number, payment, and extra pay from \
this screenshot. Use 0 for extra pay when the screenshot shows none.";

pub const SUPPORTED_MEDIA_TYPES: &[&str] = &["image/jpeg", "image/png", "image/gif", "image/webp"];

/// Response shape matches the Bun backend: `{ date, storeName, payment, extraPay }`.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedLoad {
    pub date: NaiveDate,
    pub store_name: String,
    #[serde(with = "money::number")]
    pub payment: Decimal,
    #[serde(with = "money::number")]
    pub extra_pay: Decimal,
}

#[derive(Deserialize)]
struct MessagesResponse {
    stop_reason: Option<String>,
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
}

fn upstream_failed() -> AppError {
    AppError::BadGateway("Could not read the screenshot".into())
}

pub async fn extract_load(
    state: &AppState,
    media_type: &str,
    image: &[u8],
) -> Result<ExtractedLoad, AppError> {
    let body = json!({
        "model": MODEL,
        "max_tokens": 16000,
        "fallbacks": "default",
        "output_config": {
            "format": {
                "type": "json_schema",
                "schema": {
                    "type": "object",
                    "properties": {
                        "date": { "type": "string", "format": "date" },
                        "storeName": { "type": "string" },
                        "payment": { "type": "number" },
                        "extraPay": { "type": "number" },
                    },
                    "required": ["date", "storeName", "payment", "extraPay"],
                    "additionalProperties": false,
                },
            },
        },
        "messages": [{
            "role": "user",
            "content": [
                {
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": media_type,
                        "data": base64::engine::general_purpose::STANDARD.encode(image),
                    },
                },
                { "type": "text", "text": PROMPT },
            ],
        }],
    });

    let url = format!(
        "{}/v1/messages",
        state.config.anthropic_base_url.trim_end_matches('/')
    );
    let response = state
        .http
        .post(url)
        .header("x-api-key", &state.config.anthropic_api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("anthropic-beta", FALLBACK_BETA)
        .json(&body)
        .send()
        .await
        .map_err(|error| {
            tracing::error!(%error, "Claude API request failed");
            upstream_failed()
        })?;

    let status = response.status();
    if !status.is_success() {
        let detail = response.text().await.unwrap_or_default();
        tracing::error!(status = status.as_u16(), detail = %truncate(&detail), "Claude API error");
        return Err(upstream_failed());
    }

    let message: MessagesResponse = response.json().await.map_err(|error| {
        tracing::error!(%error, "unreadable Claude API response");
        upstream_failed()
    })?;

    // A refusal (even after fallbacks) or a truncated reply may not match the schema.
    match message.stop_reason.as_deref() {
        Some("end_turn") => {}
        other => {
            tracing::warn!(stop_reason = ?other, "screenshot extraction did not complete");
            return Err(upstream_failed());
        }
    }

    // Thinking blocks come first; the structured output is the text block.
    let text = message
        .content
        .iter()
        .find(|block| block.kind == "text")
        .and_then(|block| block.text.as_deref())
        .ok_or_else(upstream_failed)?;

    let extracted: ExtractedLoad = serde_json::from_str(text).map_err(|error| {
        tracing::warn!(%error, text = %truncate(text), "extraction didn't match the schema");
        upstream_failed()
    })?;

    normalize(extracted)
}

/// The schema can't express "non-negative, 2 decimal places", so enforce it here. The values
/// only pre-fill the load form, where the admin reviews them before saving.
fn normalize(mut load: ExtractedLoad) -> Result<ExtractedLoad, AppError> {
    let cents =
        |value: Decimal| value.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
    load.payment = cents(load.payment);
    load.extra_pay = cents(load.extra_pay);
    load.store_name = load.store_name.trim().to_owned();

    if load.payment.is_sign_negative() || load.extra_pay.is_sign_negative() {
        tracing::warn!("extraction returned a negative amount");
        return Err(upstream_failed());
    }
    Ok(load)
}

fn truncate(text: &str) -> &str {
    match text.char_indices().nth(500) {
        Some((index, _)) => &text[..index],
        None => text,
    }
}

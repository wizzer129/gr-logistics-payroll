//! Screenshot extraction against a mock Claude API (wiremock) — no real API calls.

mod common;

use axum::{Router, body::Body, http::Request};
use common::{app_with_db_and_config, insert_user, login, test_config};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path},
};

const IMAGE: &[u8] = b"\x89PNG fake image bytes";

struct Ctx {
    app: Router,
    cookie: String,
    claude: MockServer,
}

async fn ctx(pool: PgPool) -> Ctx {
    let claude = MockServer::start().await;
    let mut config = test_config();
    config.anthropic_base_url = claude.uri();
    let app = app_with_db_and_config(pool.clone(), config).await;
    insert_user(&pool, "admin@example.com", "admin").await;
    let cookie = login(&app, "admin@example.com").await;
    Ctx {
        app,
        cookie,
        claude,
    }
}

/// A Messages API reply: an (empty, omitted-display) thinking block, then the JSON text.
fn claude_reply(text: &str, stop_reason: &str) -> Value {
    json!({
        "id": "msg_test",
        "type": "message",
        "role": "assistant",
        "model": "claude-opus-5-5",
        "content": [
            { "type": "thinking", "thinking": "", "signature": "sig" },
            { "type": "text", "text": text },
        ],
        "stop_reason": stop_reason,
        "usage": { "input_tokens": 1, "output_tokens": 1 },
    })
}

async fn mock_claude(server: &MockServer, response: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(response)
        .mount(server)
        .await;
}

async fn upload(ctx: &Ctx, field: &str, content_type: &str, bytes: &[u8]) -> (u16, Value) {
    let mut body = format!(
        "--BOUNDARY\r\nContent-Disposition: form-data; name=\"{field}\"; filename=\"shot\"\r\n\
         Content-Type: {content_type}\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(bytes);
    body.extend_from_slice(b"\r\n--BOUNDARY--\r\n");

    let request = Request::builder()
        .method("POST")
        .uri("/api/loads/extract")
        .header("cookie", &ctx.cookie)
        .header("content-type", "multipart/form-data; boundary=BOUNDARY")
        .body(Body::from(body))
        .unwrap();
    let response = ctx.app.clone().oneshot(request).await.unwrap();
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[sqlx::test(migrations = false)]
async fn extracts_and_sends_the_expected_request(pool: PgPool) {
    let ctx = ctx(pool).await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "test"))
        .and(header("anthropic-version", "2023-06-01"))
        .and(header("anthropic-beta", "server-side-fallback-2026-07-01"))
        .respond_with(ResponseTemplate::new(200).set_body_json(claude_reply(
            r#"{"date":"2026-10-05","storeName":" Walmart 12 ","payment":120.555,"extraPay":0}"#,
            "end_turn",
        )))
        .expect(1)
        .mount(&ctx.claude)
        .await;

    let (status, body) = upload(&ctx, "screenshot", "image/png", IMAGE).await;

    assert_eq!(status, 200, "{body}");
    assert_eq!(body["date"], "2026-10-05");
    assert_eq!(body["storeName"], "Walmart 12");
    assert_eq!(
        body["payment"].to_string(),
        "120.56",
        "rounded half-up to cents"
    );
    assert_eq!(body["extraPay"].as_f64(), Some(0.0));

    let requests = ctx.claude.received_requests().await.unwrap();
    let sent: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(sent["model"], "claude-opus-5-5");
    assert_eq!(sent["fallbacks"], "default");
    assert_eq!(sent["output_config"]["format"]["type"], "json_schema");
    let image = &sent["messages"][0]["content"][0];
    assert_eq!(image["source"]["media_type"], "image/png");
    use base64::Engine;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(image["source"]["data"].as_str().unwrap())
        .unwrap();
    assert_eq!(decoded, IMAGE);
}

#[sqlx::test(migrations = false)]
async fn upstream_failures_are_502(pool: PgPool) {
    let cases = [
        ResponseTemplate::new(529).set_body_string("overloaded"),
        ResponseTemplate::new(200).set_body_json(claude_reply("", "refusal")),
        ResponseTemplate::new(200).set_body_json(claude_reply(r#"{"date":"2026-10"#, "max_tokens")),
        ResponseTemplate::new(200).set_body_json(claude_reply("not json", "end_turn")),
        ResponseTemplate::new(200).set_body_json(claude_reply(
            r#"{"date":"2026-10-05","storeName":"A","payment":-5,"extraPay":0}"#,
            "end_turn",
        )),
        ResponseTemplate::new(200).set_body_string("<html>proxy error</html>"),
    ];

    let ctx = ctx(pool).await;
    for (index, response) in cases.into_iter().enumerate() {
        ctx.claude.reset().await;
        mock_claude(&ctx.claude, response).await;

        let (status, body) = upload(&ctx, "screenshot", "image/png", IMAGE).await;
        assert_eq!(status, 502, "case {index}: {body}");
        assert_eq!(
            body["error"], "Could not read the screenshot",
            "case {index}"
        );
    }
}

#[sqlx::test(migrations = false)]
async fn bad_uploads_are_400_without_calling_claude(pool: PgPool) {
    let ctx = ctx(pool).await;
    mock_claude(&ctx.claude, ResponseTemplate::new(500)).await;

    let (status, body) = upload(&ctx, "screenshot", "application/pdf", IMAGE).await;
    assert_eq!(status, 400);
    assert_eq!(
        body["error"],
        "Unsupported screenshot type: application/pdf"
    );

    let (status, body) = upload(&ctx, "other", "image/png", IMAGE).await;
    assert_eq!(status, 400);
    assert_eq!(body["error"], "Missing screenshot file");

    let (status, _) = upload(&ctx, "screenshot", "image/png", b"").await;
    assert_eq!(status, 400);

    let json_body = Request::builder()
        .method("POST")
        .uri("/api/loads/extract")
        .header("cookie", &ctx.cookie)
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .unwrap();
    let response = ctx.app.clone().oneshot(json_body).await.unwrap();
    assert_eq!(response.status(), 400);

    assert!(ctx.claude.received_requests().await.unwrap().is_empty());
}

#[sqlx::test(migrations = false)]
async fn oversized_screenshot_is_413(pool: PgPool) {
    let ctx = ctx(pool).await;
    let too_big = vec![0u8; 11 * 1024 * 1024];
    let (status, _) = upload(&ctx, "screenshot", "image/png", &too_big).await;
    assert_eq!(status, 413);
}

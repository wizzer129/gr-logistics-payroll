//! A queued [`Message`] → what to render and attach (EMAIL-SPEC.md §5). Runs at send time,
//! so a pay stub reflects its log as it is when the email goes out.

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{message::Message, render::Renderer, sender::Attachment};
use crate::{
	locale::Locale,
	pdf,
	services::{monthly_report, paystubs},
};

pub struct Composed {
	pub template: &'static str,
	pub context: Value,
	pub attachments: Vec<Attachment>,
}

pub enum ComposeError {
	/// Will fail the same way every time (the log was deleted, the PDF won't build…).
	Permanent(String),
	/// Worth retrying once the database is reachable.
	Database(sqlx::Error),
}

impl From<sqlx::Error> for ComposeError {
	fn from(error: sqlx::Error) -> Self {
		Self::Database(error)
	}
}

pub async fn compose(
	pool: &PgPool,
	renderer: &Renderer,
	message: &Message,
	locale: Locale,
) -> Result<Composed, ComposeError> {
	let template = message.template();
	let simple = |context| Composed {
		template,
		context,
		attachments: Vec::new(),
	};

	match message {
		Message::TestEmail { sent_at } => Ok(simple(json!({ "sent_at": sent_at }))),
		Message::PasswordReset { reset_url } => Ok(simple(json!({ "reset_url": reset_url }))),
		Message::Paystub { paycheck_log_id } => {
			let source = paystubs::source(pool, *paycheck_log_id)
				.await?
				.ok_or_else(|| ComposeError::Permanent("paycheck log not found".into()))?;
			let strings = renderer.strings(locale);
			let document = pdf::paystub::document(&source, locale, &strings["paystub_pdf"]);
			let context = json!({
				"employee_name": document.employee_name,
				"week": document.week,
				"payday": document.payday,
				"total_pay": document.rounded_pay,
			});
			let filename = format!(
				"{}-{}-{}.pdf",
				strings["paystub"]["filename"]
					.as_str()
					.unwrap_or("pay-stub"),
				slug(&source.employee_name),
				source.week_start,
			);

			let bytes = render_pdf(move || pdf::paystub(&document)).await?;
			Ok(Composed {
				template,
				context,
				attachments: vec![pdf_attachment(filename, bytes)],
			})
		}
		Message::MonthlyReport { month } => {
			let source = monthly_report::source(pool, *month).await?;
			let strings = renderer.strings(locale);
			let document =
				pdf::monthly_report::document(&source, locale, &strings["monthly_report_pdf"]);
			let context = json!({
				"month": document.month,
				"total_paid": document.paid.value,
				"paid_change": document.paid.change.as_ref()
					.map(|change| format!("{change} {}", document.vs_previous))
					.unwrap_or_default(),
				"gross": document.gross.value,
				"paychecks": document.paychecks.value,
				"drafts_note": document.drafts_note.clone().unwrap_or_default(),
			});
			let filename = format!(
				"{}-{}.pdf",
				strings["monthly_report"]["filename"]
					.as_str()
					.unwrap_or("monthly-report"),
				month.format("%Y-%m"),
			);
			let bytes = render_pdf(move || pdf::monthly_report(&document)).await?;
			Ok(Composed {
				template,
				context,
				attachments: vec![pdf_attachment(filename, bytes)],
			})
		}
	}
}

/// Typst is CPU-bound, so it runs on the blocking pool.
async fn render_pdf(
	render: impl FnOnce() -> Result<Vec<u8>, String> + Send + 'static,
) -> Result<Vec<u8>, ComposeError> {
	tokio::task::spawn_blocking(render)
		.await
		.map_err(|error| ComposeError::Permanent(format!("pdf task failed: {error}")))?
		.map_err(ComposeError::Permanent)
}

fn pdf_attachment(filename: String, bytes: Vec<u8>) -> Attachment {
	Attachment {
		filename,
		content_type: "application/pdf".into(),
		bytes,
	}
}

/// `Ana Díaz` → `ana-diaz`, for attachment file names.
fn slug(name: &str) -> String {
	let mut slug = String::new();
	for c in name.to_lowercase().chars() {
		let c = match c {
			'á' | 'à' | 'ä' | 'â' => 'a',
			'é' | 'è' | 'ë' | 'ê' => 'e',
			'í' | 'ì' | 'ï' | 'î' => 'i',
			'ó' | 'ò' | 'ö' | 'ô' => 'o',
			'ú' | 'ù' | 'ü' | 'û' => 'u',
			'ñ' => 'n',
			c => c,
		};
		if c.is_ascii_alphanumeric() {
			slug.push(c);
		} else if !slug.ends_with('-') {
			slug.push('-');
		}
	}
	slug.trim_matches('-').to_owned()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn slugs_are_ascii_and_dashed() {
		assert_eq!(slug("Ana Díaz"), "ana-diaz");
		assert_eq!(slug("  José  Núñez-Peña "), "jose-nunez-pena");
	}
}

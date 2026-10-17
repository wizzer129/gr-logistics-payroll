//! What can be emailed. A [`Message`] is the outbox payload: it says _what_ to send, and the
//! worker turns it into an email at send time (EMAIL-SPEC.md §5).

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use crate::locale::Locale;

/// One variant per kind of email. The serialized tag is the outbox `kind` and must match
/// that column's CHECK constraint (migration 0010).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Message {
	/// `gr-logistics send-test-email` and the outbox tests.
	TestEmail { sent_at: String },
	/// The link carries a live token, so the outbox scrubs this payload once it's sent.
	PasswordReset { reset_url: String },
	/// Rendered from the log as it is when the email goes out, with the PDF attached.
	Paystub { paycheck_log_id: Uuid },
	/// The month starting on `month` (always the 1st), with the PDF attached. Built from the
	/// data as it is at send time.
	MonthlyReport { month: NaiveDate },
}

impl Message {
	pub fn kind(&self) -> &'static str {
		match self {
			Self::TestEmail { .. } => "test_email",
			Self::PasswordReset { .. } => "password_reset",
			Self::Paystub { .. } => "paystub",
			Self::MonthlyReport { .. } => "monthly_report",
		}
	}

	/// The Maizzle template (`templates/email/<name>.html` / `.txt`) and the locale-file
	/// section its strings live under.
	pub fn template(&self) -> &'static str {
		match self {
			Self::TestEmail { .. } => "test-email",
			Self::PasswordReset { .. } => "password-reset",
			Self::Paystub { .. } => "paystub",
			Self::MonthlyReport { .. } => "monthly-report",
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_serialized_tag_is_the_outbox_kind() {
		let message = Message::TestEmail {
			sent_at: "now".into(),
		};
		let payload = serde_json::to_value(&message).unwrap();
		assert_eq!(payload["kind"], message.kind());
		assert_eq!(serde_json::from_value::<Message>(payload).unwrap(), message);
	}
}

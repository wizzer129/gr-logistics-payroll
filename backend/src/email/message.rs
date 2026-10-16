//! What can be emailed. A [`Message`] is the outbox payload: it says _what_ to send, and the
//! worker turns it into an email at send time (EMAIL-SPEC.md §5).

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// The language an email is rendered in: its recipient's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
	En,
	Es,
}

impl Locale {
	pub const ALL: [Self; 2] = [Self::En, Self::Es];

	pub fn as_str(self) -> &'static str {
		match self {
			Self::En => "en",
			Self::Es => "es",
		}
	}

	pub fn parse(value: &str) -> Option<Self> {
		match value {
			"en" => Some(Self::En),
			"es" => Some(Self::Es),
			_ => None,
		}
	}
}

/// One variant per kind of email. The serialized tag is the outbox `kind` and must match
/// that column's CHECK constraint (migration 0010).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Message {
	/// `gr-logistics send-test-email` and the outbox tests.
	TestEmail { sent_at: String },
	/// The link carries a live token, so the outbox scrubs this payload once it's sent.
	PasswordReset { reset_url: String },
}

impl Message {
	pub fn kind(&self) -> &'static str {
		match self {
			Self::TestEmail { .. } => "test_email",
			Self::PasswordReset { .. } => "password_reset",
		}
	}

	/// The Maizzle template (`templates/email/<name>.html` / `.txt`) and the locale-file
	/// section its strings live under.
	pub fn template(&self) -> &'static str {
		match self {
			Self::TestEmail { .. } => "test-email",
			Self::PasswordReset { .. } => "password-reset",
		}
	}

	/// The template variables besides `t` (the locale strings). Everything is a string:
	/// money and dates arrive here already formatted.
	pub fn context(&self) -> Value {
		match self {
			Self::TestEmail { sent_at } => json!({ "sent_at": sent_at }),
			Self::PasswordReset { reset_url } => json!({ "reset_url": reset_url }),
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

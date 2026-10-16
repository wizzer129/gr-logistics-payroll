//! Outgoing email (EMAIL-SPEC.md). Handlers only ever call [`enqueue`], inside the
//! transaction that caused the email; the [`outbox`] worker renders and sends it later.

pub mod message;
pub mod outbox;
pub mod render;
pub mod sender;

use sqlx::PgConnection;
use uuid::Uuid;

pub use message::{Locale, Message};

use crate::services::email_outbox::{self, NewOutboxEmail};

/// Queues `message` for `recipient`. Nothing is sent until the caller's transaction commits.
pub async fn enqueue(
	conn: &mut PgConnection,
	message: &Message,
	recipient: &str,
	locale: Locale,
	requested_by: Option<Uuid>,
) -> Result<Uuid, sqlx::Error> {
	let payload = serde_json::to_value(message).expect("a Message always serializes");
	email_outbox::insert(
		conn,
		NewOutboxEmail {
			kind: message.kind(),
			recipient,
			locale: locale.as_str(),
			payload: &payload,
			requested_by,
		},
	)
	.await
}

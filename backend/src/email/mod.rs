//! Outgoing email (EMAIL-SPEC.md). Handlers only ever call [`enqueue`], inside the
//! transaction that caused the email; the [`outbox`] worker renders and sends it later.

pub mod compose;
pub mod message;
pub mod outbox;
pub mod render;
pub mod scheduler;
pub mod sender;

use sqlx::PgConnection;
use uuid::Uuid;

pub use message::Message;

use crate::{
	locale::Locale,
	services::{
		email_outbox::{self, NewOutboxEmail},
		email_recipients::Recipient,
	},
};

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

/// [`enqueue`] once per recipient, each in their own language; returns the outbox ids.
pub async fn enqueue_for_each(
	conn: &mut PgConnection,
	message: &Message,
	recipients: &[Recipient],
	requested_by: Option<Uuid>,
) -> Result<Vec<Uuid>, sqlx::Error> {
	let mut ids = Vec::with_capacity(recipients.len());
	for recipient in recipients {
		ids.push(
			enqueue(
				conn,
				message,
				&recipient.email,
				recipient.locale,
				requested_by,
			)
			.await?,
		);
	}
	Ok(ids)
}

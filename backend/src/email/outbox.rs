//! The outbox worker (EMAIL-SPEC.md §5): one task in `serve` that sends queued emails, with
//! retries, and records how each one went.

use std::{sync::Arc, time::Duration};

use chrono::Utc;
use sqlx::{PgPool, postgres::PgListener};
use tokio::sync::watch;
use uuid::Uuid;

use super::{
	compose::{ComposeError, compose},
	message::Message,
	render::Renderer,
	sender::{EmailSender, OutgoingEmail, SendError},
};
use crate::{
	locale::Locale,
	services::email_outbox::{self, ClaimedEmail, NOTIFY_CHANNEL},
};

/// After this many failed attempts a retryable error becomes `failed`. With the backoff
/// below that's retries 1, 2, 4 and 8 minutes apart: about a quarter hour in all.
pub const MAX_ATTEMPTS: i32 = 5;
const BATCH: i64 = 10;
/// Fallback wake-up, for retries coming due and in case a notification is missed.
const POLL_EVERY: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct Outbox {
	pool: PgPool,
	sender: Arc<dyn EmailSender>,
	renderer: Arc<Renderer>,
}

impl Outbox {
	pub fn new(pool: PgPool, sender: Arc<dyn EmailSender>, renderer: Arc<Renderer>) -> Self {
		Self {
			pool,
			sender,
			renderer,
		}
	}

	/// Sends every row that's due now, batch by batch; returns how many were handled. The
	/// whole loop body, so tests can drive it without timers.
	pub async fn process_due(&self) -> Result<usize, sqlx::Error> {
		let mut handled = 0;
		loop {
			let batch = email_outbox::claim_due(&self.pool, BATCH).await?;
			let full = batch.len() as i64 == BATCH;
			for row in batch {
				self.process(row).await?;
				handled += 1;
			}
			if !full {
				return Ok(handled);
			}
		}
	}

	async fn process(&self, row: ClaimedEmail) -> Result<(), sqlx::Error> {
		let email = match self.prepare(&row).await {
			Ok(email) => email,
			Err(ComposeError::Permanent(error)) => return self.failed(row.id, &error).await,
			// The claim lease runs out and the row is retried, like any database hiccup.
			Err(ComposeError::Database(error)) => return Err(error),
		};

		match self.sender.send(&email).await {
			Ok(()) => {
				tracing::info!(id = %row.id, kind = row.kind, "email sent");
				email_outbox::mark_sent(&self.pool, row.id, &email.subject).await
			}
			Err(SendError::Permanent(error)) => self.failed(row.id, &error).await,
			Err(SendError::Retryable(error)) if row.attempts + 1 >= MAX_ATTEMPTS => {
				self.failed(
					row.id,
					&format!("gave up after {MAX_ATTEMPTS} attempts: {error}"),
				)
				.await
			}
			Err(SendError::Retryable(error)) => {
				let delay = chrono::Duration::minutes(1 << row.attempts);
				tracing::warn!(id = %row.id, kind = row.kind, attempt = row.attempts + 1, %error, "email send failed; will retry");
				email_outbox::schedule_retry(&self.pool, row.id, Utc::now() + delay, &error).await
			}
		}
	}

	async fn failed(&self, id: Uuid, error: &str) -> Result<(), sqlx::Error> {
		tracing::error!(%id, error, "email failed");
		email_outbox::mark_failed(&self.pool, id, error).await
	}

	/// A row that can't be turned into an email (unreadable payload, the paycheck log is
	/// gone, a render error) is a permanent failure: retrying would fail the same way.
	async fn prepare(&self, row: &ClaimedEmail) -> Result<OutgoingEmail, ComposeError> {
		let permanent = ComposeError::Permanent;
		let payload = row
			.payload
			.as_ref()
			.ok_or_else(|| permanent("payload is missing".to_owned()))?;
		let message: Message = serde_json::from_value(payload.0.clone())
			.map_err(|error| permanent(format!("unreadable payload: {error}")))?;
		let locale = Locale::parse(&row.locale)
			.ok_or_else(|| permanent(format!("unknown locale {:?}", row.locale)))?;

		let composed = compose(&self.pool, &self.renderer, &message, locale).await?;
		let rendered = self
			.renderer
			.render(composed.template, locale, composed.context)
			.map_err(|error| permanent(format!("render failed: {error}")))?;

		Ok(OutgoingEmail {
			id: Some(row.id),
			to: row.recipient.clone(),
			subject: rendered.subject,
			html: rendered.html,
			text: rendered.text,
			attachments: composed.attachments,
		})
	}

	/// Runs until `shutdown` flips to `true`. Wakes on a commit notification or every
	/// `POLL_EVERY`; a send in flight always finishes before it exits.
	pub async fn run(self, mut shutdown: watch::Receiver<bool>) {
		let mut listener = match PgListener::connect_with(&self.pool).await {
			Ok(mut listener) => match listener.listen(NOTIFY_CHANNEL).await {
				Ok(()) => Some(listener),
				Err(error) => {
					tracing::warn!(%error, "outbox: LISTEN failed; polling only");
					None
				}
			},
			Err(error) => {
				tracing::warn!(%error, "outbox: no listener connection; polling only");
				None
			}
		};
		tracing::info!("email outbox worker started");

		loop {
			if let Err(error) = self.process_due().await {
				tracing::error!(%error, "outbox: processing failed");
			}

			let notified = async {
				match listener.as_mut() {
					Some(listener) => {
						if let Err(error) = listener.recv().await {
							// PgListener reconnects on the next recv; avoid a hot loop meanwhile.
							tracing::warn!(%error, "outbox: listener error");
							tokio::time::sleep(POLL_EVERY).await;
						}
					}
					None => std::future::pending().await,
				}
			};

			tokio::select! {
				_ = shutdown.changed() => break,
				() = notified => {},
				() = tokio::time::sleep(POLL_EVERY) => {},
			}
			if *shutdown.borrow() {
				break;
			}
		}
		tracing::info!("email outbox worker stopped");
	}
}

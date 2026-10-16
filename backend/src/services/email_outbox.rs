//! `email_outbox` (EMAIL-SPEC.md §5): enqueue in the caller's transaction, then the worker
//! claims, sends and records the outcome.

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgConnection, PgPool, types::Json};
use uuid::Uuid;

/// The `LISTEN` channel `insert` notifies on. Postgres delivers it only when the inserting
/// transaction commits, so the worker never wakes for a row it can't see.
pub const NOTIFY_CHANNEL: &str = "email_outbox";

/// How long a claimed row is hidden from other claims while it's being sent. A worker that
/// dies mid-send leaves the row to be retried once this runs out.
const CLAIM_LEASE: &str = "5 minutes";

pub struct NewOutboxEmail<'a> {
	pub kind: &'a str,
	pub recipient: &'a str,
	pub locale: &'a str,
	pub payload: &'a Value,
	pub requested_by: Option<Uuid>,
}

pub async fn insert(
	conn: &mut PgConnection,
	email: NewOutboxEmail<'_>,
) -> Result<Uuid, sqlx::Error> {
	let id: Uuid = sqlx::query_scalar(
		"INSERT INTO email_outbox (kind, recipient, locale, payload, requested_by)
		 VALUES ($1, $2, $3, $4, $5)
		 RETURNING id",
	)
	.bind(email.kind)
	.bind(email.recipient)
	.bind(email.locale)
	.bind(Json(email.payload))
	.bind(email.requested_by)
	.fetch_one(&mut *conn)
	.await?;

	sqlx::query("SELECT pg_notify($1, $2)")
		.bind(NOTIFY_CHANNEL)
		.bind(id.to_string())
		.execute(&mut *conn)
		.await?;

	Ok(id)
}

#[derive(sqlx::FromRow)]
pub struct ClaimedEmail {
	pub id: Uuid,
	pub kind: String,
	pub recipient: String,
	pub locale: String,
	pub payload: Option<Json<Value>>,
	pub attempts: i32,
}

/// Claims up to `limit` due rows by pushing their `next_attempt_at` past the lease, in one
/// statement. `SKIP LOCKED` keeps concurrent workers from claiming the same row, and nothing
/// stays locked while the (slow) send happens.
pub async fn claim_due(pool: &PgPool, limit: i64) -> Result<Vec<ClaimedEmail>, sqlx::Error> {
	sqlx::query_as(
		"UPDATE email_outbox
		 SET next_attempt_at = now() + $2::interval
		 WHERE id IN (
			SELECT id FROM email_outbox
			WHERE status = 'pending' AND next_attempt_at <= now()
			ORDER BY next_attempt_at
			LIMIT $1
			FOR UPDATE SKIP LOCKED
		 )
		 RETURNING id, kind, recipient, locale, payload, attempts",
	)
	.bind(limit)
	.bind(CLAIM_LEASE)
	.fetch_all(pool)
	.await
}

pub async fn mark_sent(pool: &PgPool, id: Uuid, subject: &str) -> Result<(), sqlx::Error> {
	sqlx::query(
		"UPDATE email_outbox
		 SET status = 'sent', sent_at = now(), subject = $2, attempts = attempts + 1,
			last_error = NULL,
			payload = CASE WHEN kind = 'password_reset' THEN NULL ELSE payload END
		 WHERE id = $1",
	)
	.bind(id)
	.bind(subject)
	.execute(pool)
	.await?;
	Ok(())
}

pub async fn schedule_retry(
	pool: &PgPool,
	id: Uuid,
	next_attempt_at: DateTime<Utc>,
	error: &str,
) -> Result<(), sqlx::Error> {
	sqlx::query(
		"UPDATE email_outbox
		 SET attempts = attempts + 1, next_attempt_at = $2, last_error = $3
		 WHERE id = $1",
	)
	.bind(id)
	.bind(next_attempt_at)
	.bind(error)
	.execute(pool)
	.await?;
	Ok(())
}

pub async fn mark_failed(pool: &PgPool, id: Uuid, error: &str) -> Result<(), sqlx::Error> {
	sqlx::query(
		"UPDATE email_outbox
		 SET status = 'failed', attempts = attempts + 1, last_error = $2,
			payload = CASE WHEN kind = 'password_reset' THEN NULL ELSE payload END
		 WHERE id = $1",
	)
	.bind(id)
	.bind(error)
	.execute(pool)
	.await?;
	Ok(())
}

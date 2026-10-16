//! `password_reset_tokens` plus the user/session writes a reset makes (EMAIL-SPEC.md §7).

use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

/// The user a reset request is for, matched the same way login matches (exact email).
pub async fn user_id_by_email(
	conn: &mut PgConnection,
	email: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
	sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
		.bind(email)
		.fetch_optional(conn)
		.await
}

/// Tokens issued to `user_id` in the last hour, for the silent rate limit.
pub async fn recent_count(conn: &mut PgConnection, user_id: Uuid) -> Result<i64, sqlx::Error> {
	sqlx::query_scalar(
		"SELECT count(*) FROM password_reset_tokens
		 WHERE user_id = $1 AND created_at > now() - interval '1 hour'",
	)
	.bind(user_id)
	.fetch_one(conn)
	.await
}

/// Retires the user's unused tokens and issues a new one: only the latest link works.
pub async fn issue(
	conn: &mut PgConnection,
	user_id: Uuid,
	token_hash: &[u8],
	expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	sqlx::query(
		"UPDATE password_reset_tokens SET used_at = now()
		 WHERE user_id = $1 AND used_at IS NULL",
	)
	.bind(user_id)
	.execute(&mut *conn)
	.await?;

	sqlx::query(
		"INSERT INTO password_reset_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)",
	)
	.bind(user_id)
	.bind(token_hash)
	.bind(expires_at)
	.execute(&mut *conn)
	.await?;
	Ok(())
}

/// Marks a valid (unused, unexpired) token used and returns its user. `FOR UPDATE` makes two
/// simultaneous uses of one link race for the row: only one gets it.
pub async fn consume(
	conn: &mut PgConnection,
	token_hash: &[u8],
) -> Result<Option<Uuid>, sqlx::Error> {
	let row: Option<(Uuid, Uuid)> = sqlx::query_as(
		"SELECT id, user_id FROM password_reset_tokens
		 WHERE token_hash = $1 AND used_at IS NULL AND expires_at > now()
		 FOR UPDATE",
	)
	.bind(token_hash)
	.fetch_optional(&mut *conn)
	.await?;

	let Some((id, user_id)) = row else {
		return Ok(None);
	};
	sqlx::query("UPDATE password_reset_tokens SET used_at = now() WHERE id = $1")
		.bind(id)
		.execute(&mut *conn)
		.await?;
	Ok(Some(user_id))
}

/// Sets the new password and signs the user out everywhere.
pub async fn set_password_and_revoke_sessions(
	conn: &mut PgConnection,
	user_id: Uuid,
	password_hash: &str,
) -> Result<(), sqlx::Error> {
	sqlx::query("UPDATE users SET password_hash = $2 WHERE id = $1")
		.bind(user_id)
		.bind(password_hash)
		.execute(&mut *conn)
		.await?;
	sqlx::query("DELETE FROM sessions WHERE user_id = $1")
		.bind(user_id)
		.execute(&mut *conn)
		.await?;
	Ok(())
}

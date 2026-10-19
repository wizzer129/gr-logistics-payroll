//! Sessions in the `sessions` table plus the `gr_session` cookie (HttpOnly, SameSite=Lax,
//! Secure in production).

use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	auth::{AuthUser, Role},
	locale::Locale,
	theme::Theme,
};

pub const SESSION_COOKIE: &str = "gr_session";

pub async fn create_session(
	pool: &PgPool,
	user_id: Uuid,
	ttl_hours: i64,
) -> Result<(Uuid, DateTime<Utc>), sqlx::Error> {
	let expires_at = Utc::now() + Duration::hours(ttl_hours);

	let session_id = sqlx::query_scalar(
		"INSERT INTO sessions (user_id, expires_at) VALUES ($1, $2) RETURNING session_id",
	)
	.bind(user_id)
	.bind(expires_at)
	.fetch_one(pool)
	.await?;

	Ok((session_id, expires_at))
}

/// `None` for an unknown or expired session, or a user whose role this build doesn't know.
pub async fn user_by_session_id(
	pool: &PgPool,
	session_id: Uuid,
) -> Result<Option<AuthUser>, sqlx::Error> {
	let row: Option<(Uuid, String, String, String, String)> = sqlx::query_as(
		"SELECT users.id, users.email, users.role, users.locale, users.theme
         FROM sessions
         JOIN users ON users.id = sessions.user_id
         WHERE sessions.session_id = $1
         AND sessions.expires_at > now()",
	)
	.bind(session_id)
	.fetch_optional(pool)
	.await?;

	Ok(row.and_then(|(id, email, role, locale, theme)| {
		let Some(role) = Role::parse(&role) else {
			tracing::warn!(%id, role, "user has an unknown role; rejecting session");
			return None;
		};
		Some(AuthUser {
			id,
			email,
			role,
			locale: Locale::parse(&locale).unwrap_or(Locale::En),
			theme: Theme::parse(&theme).unwrap_or_default(),
		})
	}))
}

pub async fn destroy_session(pool: &PgPool, session_id: Uuid) -> Result<(), sqlx::Error> {
	sqlx::query("DELETE FROM sessions WHERE session_id = $1")
		.bind(session_id)
		.execute(pool)
		.await?;
	Ok(())
}

/// Deletes sessions past their expiry (the hourly cleanup); returns how many.
pub async fn delete_expired(pool: &PgPool) -> Result<u64, sqlx::Error> {
	let result = sqlx::query("DELETE FROM sessions WHERE expires_at <= now()")
		.execute(pool)
		.await?;
	Ok(result.rows_affected())
}

/// A cookie that isn't a UUID is treated as no session (401) rather than reaching Postgres
/// as a bad cast (a 500).
pub fn read_session_id(jar: &CookieJar) -> Option<Uuid> {
	jar.get(SESSION_COOKIE)
		.and_then(|cookie| Uuid::parse_str(cookie.value()).ok())
}

pub fn session_cookie(
	session_id: Uuid,
	expires_at: DateTime<Utc>,
	secure: bool,
) -> Cookie<'static> {
	let expires = OffsetDateTime::from_unix_timestamp(expires_at.timestamp())
		.unwrap_or(OffsetDateTime::UNIX_EPOCH);

	Cookie::build((SESSION_COOKIE, session_id.to_string()))
		.http_only(true)
		.secure(secure)
		.same_site(SameSite::Lax)
		.path("/")
		.expires(expires)
		.build()
}

/// Path must match the original cookie's for the browser to drop it.
pub fn removal_cookie() -> Cookie<'static> {
	Cookie::build((SESSION_COOKIE, "")).path("/").build()
}

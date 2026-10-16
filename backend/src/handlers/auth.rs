use axum::{Json, extract::State, http::StatusCode};
use axum_extra::extract::CookieJar;
use chrono::{Duration, Utc};

use crate::{
	auth::{AuthUser, Role, password, reset_token, session},
	email::{self, Locale, Message},
	error::{AppError, AppResult, ErrorCode},
	extract::ValidatedJson,
	models::auth::{LoginRequest, PasswordResetConfirm, PasswordResetRequest, UpdateLocale},
	services::{email_recipients, password_resets},
	state::AppState,
};

const INVALID_CREDENTIALS: &str = "Invalid email or password";

/// POST /api/auth/login — public.
pub async fn login(
	State(state): State<AppState>,
	jar: CookieJar,
	ValidatedJson(body): ValidatedJson<LoginRequest>,
) -> AppResult<(CookieJar, Json<AuthUser>)> {
	let row: Option<(uuid::Uuid, String, String, String, String)> =
		sqlx::query_as("SELECT id, email, role, password_hash, locale FROM users WHERE email = $1")
			.bind(&body.email)
			.fetch_optional(&state.pool)
			.await?;

	let Some((id, email, role, password_hash, locale)) = row else {
		password::burn_verify_time(body.password).await;
		return Err(AppError::Unauthorized(
			ErrorCode::InvalidCredentials,
			INVALID_CREDENTIALS.into(),
		));
	};

	if !password::verify_password(body.password, password_hash).await? {
		return Err(AppError::Unauthorized(
			ErrorCode::InvalidCredentials,
			INVALID_CREDENTIALS.into(),
		));
	}

	let role = Role::parse(&role).ok_or_else(|| {
		tracing::warn!(%id, role, "login for user with unknown role");
		AppError::Unauthorized(ErrorCode::InvalidCredentials, INVALID_CREDENTIALS.into())
	})?;

	let (session_id, expires_at) =
		session::create_session(&state.pool, id, state.config.session_ttl_hours).await?;

	let cookie = session::session_cookie(session_id, expires_at, state.config.is_production());

	let locale = Locale::parse(&locale).unwrap_or(Locale::En);
	Ok((
		jar.add(cookie),
		Json(AuthUser {
			id,
			email,
			role,
			locale,
		}),
	))
}

/// POST /api/auth/logout — public and idempotent: always clears the cookie.
pub async fn logout(
	State(state): State<AppState>,
	jar: CookieJar,
) -> AppResult<(CookieJar, StatusCode)> {
	if let Some(session_id) = session::read_session_id(&jar) {
		session::destroy_session(&state.pool, session_id).await?;
	}

	Ok((
		jar.remove(session::removal_cookie()),
		StatusCode::NO_CONTENT,
	))
}

/// GET /api/auth/me — session required, no role check.
pub async fn me(user: AuthUser) -> Json<AuthUser> {
	Json(user)
}

/// PUT /api/auth/me/locale — any signed-in role: it only touches the caller's own row. The
/// frontend calls it when the user switches language, so their emails follow the UI.
pub async fn update_locale(
	user: AuthUser,
	State(state): State<AppState>,
	ValidatedJson(body): ValidatedJson<UpdateLocale>,
) -> AppResult<StatusCode> {
	user.require_role(&[Role::Admin, Role::Employee])?;
	let locale = Locale::parse(&body.locale).unwrap_or(Locale::En);
	email_recipients::set_user_locale(&state.pool, user.id, locale).await?;
	Ok(StatusCode::NO_CONTENT)
}

/// How long an emailed reset link works.
const RESET_TOKEN_TTL: Duration = Duration::hours(1);
/// Requests per account per hour that actually send an email; extra ones are dropped
/// silently, so the response still can't reveal anything.
const RESET_REQUESTS_PER_HOUR: i64 = 3;

/// POST /api/auth/password-reset — public. Always 202, whether or not the account exists, so
/// it can't be used to find out who has one. The email is only queued, so response time
/// doesn't give that away either.
pub async fn request_password_reset(
	State(state): State<AppState>,
	ValidatedJson(body): ValidatedJson<PasswordResetRequest>,
) -> AppResult<StatusCode> {
	let locale = body
		.locale
		.as_deref()
		.and_then(Locale::parse)
		.unwrap_or(Locale::En);
	let mut tx = state.pool.begin().await?;

	let Some(user_id) = password_resets::user_id_by_email(&mut tx, &body.email).await? else {
		return Ok(StatusCode::ACCEPTED);
	};
	if password_resets::recent_count(&mut tx, user_id).await? >= RESET_REQUESTS_PER_HOUR {
		tracing::warn!(%user_id, "password reset rate limit hit; no email sent");
		return Ok(StatusCode::ACCEPTED);
	}

	let (token, token_hash) = reset_token::generate();
	password_resets::issue(&mut tx, user_id, &token_hash, Utc::now() + RESET_TOKEN_TTL).await?;

	// CORS_ORIGIN is exactly the frontend's origin, so it's also the link's base.
	let origin = state.config.cors_origin.to_str().unwrap_or_default();
	let reset_url = format!(
		"{}/reset-password?token={token}",
		origin.trim_end_matches('/')
	);
	email::enqueue(
		&mut tx,
		&Message::PasswordReset { reset_url },
		&body.email,
		locale,
		Some(user_id),
	)
	.await?;

	tx.commit().await?;
	tracing::info!(%user_id, "password reset requested");
	Ok(StatusCode::ACCEPTED)
}

/// POST /api/auth/password-reset/confirm — public. Unknown, used and expired tokens all get
/// the same `RESET_TOKEN_INVALID`. A reset signs the user out on every device.
pub async fn confirm_password_reset(
	State(state): State<AppState>,
	ValidatedJson(body): ValidatedJson<PasswordResetConfirm>,
) -> AppResult<StatusCode> {
	// Hashed before the transaction: argon2 takes tens of ms, and the row lock shouldn't.
	let password_hash = password::hash_password(body.new_password).await?;
	let mut tx = state.pool.begin().await?;

	let Some(user_id) = password_resets::consume(&mut tx, &reset_token::hash(&body.token)).await?
	else {
		return Err(AppError::BadRequest(
			ErrorCode::ResetTokenInvalid,
			"Reset link is invalid or has expired".into(),
		));
	};
	password_resets::set_password_and_revoke_sessions(&mut tx, user_id, &password_hash).await?;

	tx.commit().await?;
	tracing::info!(%user_id, "password reset; all sessions revoked");
	Ok(StatusCode::NO_CONTENT)
}

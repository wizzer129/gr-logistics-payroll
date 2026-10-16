use axum::{Json, extract::State, http::StatusCode};
use axum_extra::extract::CookieJar;

use crate::{
	auth::{AuthUser, Role, password, session},
	error::{AppError, AppResult},
	extract::ValidatedJson,
	models::auth::LoginRequest,
	state::AppState,
};

const INVALID_CREDENTIALS: &str = "Invalid email or password";

/// POST /api/auth/login — public.
pub async fn login(
	State(state): State<AppState>,
	jar: CookieJar,
	ValidatedJson(body): ValidatedJson<LoginRequest>,
) -> AppResult<(CookieJar, Json<AuthUser>)> {
	let row: Option<(uuid::Uuid, String, String, String)> =
		sqlx::query_as("SELECT id, email, role, password_hash FROM users WHERE email = $1")
			.bind(&body.email)
			.fetch_optional(&state.pool)
			.await?;

	let Some((id, email, role, password_hash)) = row else {
		password::burn_verify_time(body.password).await;
		return Err(AppError::Unauthorized(INVALID_CREDENTIALS.into()));
	};

	if !password::verify_password(body.password, password_hash).await? {
		return Err(AppError::Unauthorized(INVALID_CREDENTIALS.into()));
	}

	let role = Role::parse(&role).ok_or_else(|| {
		tracing::warn!(%id, role, "login for user with unknown role");
		AppError::Unauthorized(INVALID_CREDENTIALS.into())
	})?;

	let (session_id, expires_at) =
		session::create_session(&state.pool, id, state.config.session_ttl_hours).await?;

	let cookie = session::session_cookie(session_id, expires_at, state.config.is_production());

	Ok((jar.add(cookie), Json(AuthUser { id, email, role })))
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

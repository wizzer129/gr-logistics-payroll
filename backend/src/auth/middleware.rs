use axum::{
	extract::{Request, State},
	middleware::Next,
	response::Response,
};
use axum_extra::extract::CookieJar;

use crate::{
	auth::session,
	error::{AppError, ErrorCode},
	state::AppState,
};

/// Resolves the `gr_session` cookie to an `AuthUser` and puts it in request extensions,
/// where the `AuthUser` extractor reads it. No valid session → 401 before any handler runs.
pub async fn require_session(
	State(state): State<AppState>,
	jar: CookieJar,
	mut request: Request,
	next: Next,
) -> Result<Response, AppError> {
	let session_id = session::read_session_id(&jar).ok_or_else(|| {
		AppError::Unauthorized(ErrorCode::NotAuthenticated, "Not authenticated".into())
	})?;

	let user = session::user_by_session_id(&state.pool, session_id)
		.await?
		.ok_or_else(|| {
			AppError::Unauthorized(
				ErrorCode::SessionExpired,
				"Session expired or invalid".into(),
			)
		})?;

	request.extensions_mut().insert(user);
	Ok(next.run(request).await)
}

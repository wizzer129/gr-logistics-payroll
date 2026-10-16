use axum::{
	Json,
	http::StatusCode,
	response::{IntoResponse, Response},
};
use serde_json::json;

pub type AppResult<T> = Result<T, AppError>;

/// The single place status codes and the `{ "error": msg }` body shape are decided.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
	#[error("{0}")]
	BadRequest(String),
	#[error("{0}")]
	Unauthorized(String),
	#[error("{0}")]
	Forbidden(String),
	#[error("{0}")]
	NotFound(String),
	#[error("{0}")]
	Conflict(String),
	#[error(transparent)]
	Internal(#[from] anyhow::Error),
}

impl From<sqlx::Error> for AppError {
	fn from(error: sqlx::Error) -> Self {
		Self::Internal(error.into())
	}
}

impl AppError {
	/// Maps a unique-constraint violation to 409 with `message`; anything else stays a 500.
	pub fn conflict_if_unique(error: sqlx::Error, message: &str) -> Self {
		match &error {
			sqlx::Error::Database(db) if db.is_unique_violation() => {
				Self::Conflict(message.to_owned())
			}
			_ => error.into(),
		}
	}
}

impl IntoResponse for AppError {
	fn into_response(self) -> Response {
		let status = match &self {
			Self::BadRequest(_) => StatusCode::BAD_REQUEST,
			Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
			Self::Forbidden(_) => StatusCode::FORBIDDEN,
			Self::NotFound(_) => StatusCode::NOT_FOUND,
			Self::Conflict(_) => StatusCode::CONFLICT,
			Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
		};

		let message = match &self {
			Self::Internal(error) => {
				tracing::error!(error = ?error, "unhandled error");
				"Internal server error".to_owned()
			}
			other => {
				tracing::warn!(status = status.as_u16(), message = %other, "request failed");
				other.to_string()
			}
		};

		(status, Json(json!({ "error": message }))).into_response()
	}
}

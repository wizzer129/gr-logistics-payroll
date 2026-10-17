use axum::{
	Json,
	http::StatusCode,
	response::{IntoResponse, Response},
};
use serde::Serialize;
use serde_json::json;

pub type AppResult<T> = Result<T, AppError>;

/// Stable, machine-readable id for every error the API returns. The frontend translates
/// by it (`errors.<CODE>` in its locale files) and shows it for troubleshooting, so a
/// variant's serialized name is part of the API contract: add new ones, never rename.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
	// 400
	InvalidBody,
	ValidationFailed,
	InvalidQuery,
	InvalidId,
	InvalidYear,
	LoadsAndLumpSum,
	NoLoads,
	WeekEndBeforeStart,
	TotalTooLarge,
	KindMismatch,
	ResetTokenInvalid,
	RecipientNotFound,
	NoRecipients,
	// 401 / 403
	NotAuthenticated,
	SessionExpired,
	InvalidCredentials,
	Forbidden,
	// 404
	NotFound,
	EmployeeNotFound,
	PaycheckLogNotFound,
	// 405
	MethodNotAllowed,
	// 409
	DuplicateWeek,
	PaycheckNotCompleted,
	// 500 / 503
	InternalError,
	RequestTimeout,
}

/// The single place status codes and the `{ "error": msg, "code": CODE }` body shape are
/// decided. `error` is English for developers and logs; the UI translates `code`.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
	#[error("{1}")]
	BadRequest(ErrorCode, String),
	#[error("{1}")]
	Unauthorized(ErrorCode, String),
	#[error("{1}")]
	Forbidden(ErrorCode, String),
	#[error("{1}")]
	NotFound(ErrorCode, String),
	#[error("{1}")]
	MethodNotAllowed(ErrorCode, String),
	#[error("{1}")]
	Conflict(ErrorCode, String),
	#[error("{1}")]
	ServiceUnavailable(ErrorCode, String),
	#[error(transparent)]
	Internal(#[from] anyhow::Error),
}

impl From<sqlx::Error> for AppError {
	fn from(error: sqlx::Error) -> Self {
		Self::Internal(error.into())
	}
}

impl AppError {
	/// Maps a unique-constraint violation to 409 with `code`/`message`; anything else stays
	/// a 500.
	pub fn conflict_if_unique(error: sqlx::Error, code: ErrorCode, message: &str) -> Self {
		match &error {
			sqlx::Error::Database(db) if db.is_unique_violation() => {
				Self::Conflict(code, message.to_owned())
			}
			_ => error.into(),
		}
	}
}

impl IntoResponse for AppError {
	fn into_response(self) -> Response {
		let (status, code) = match &self {
			Self::BadRequest(code, _) => (StatusCode::BAD_REQUEST, *code),
			Self::Unauthorized(code, _) => (StatusCode::UNAUTHORIZED, *code),
			Self::Forbidden(code, _) => (StatusCode::FORBIDDEN, *code),
			Self::NotFound(code, _) => (StatusCode::NOT_FOUND, *code),
			Self::MethodNotAllowed(code, _) => (StatusCode::METHOD_NOT_ALLOWED, *code),
			Self::Conflict(code, _) => (StatusCode::CONFLICT, *code),
			Self::ServiceUnavailable(code, _) => (StatusCode::SERVICE_UNAVAILABLE, *code),
			Self::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::InternalError),
		};

		let message = match &self {
			Self::Internal(error) => {
				tracing::error!(error = ?error, "unhandled error");
				"Internal server error".to_owned()
			}
			other => {
				tracing::warn!(status = status.as_u16(), ?code, message = %other, "request failed");
				other.to_string()
			}
		};

		(status, Json(json!({ "error": message, "code": code }))).into_response()
	}
}

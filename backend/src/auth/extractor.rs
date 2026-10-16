use axum::{extract::FromRequestParts, http::request::Parts};
use serde::Serialize;
use uuid::Uuid;

use crate::error::{AppError, ErrorCode};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
	Admin,
	/// Not provisioned yet — exists so adding it later is additive (see AGENTS.md).
	Employee,
}

impl Role {
	pub fn parse(value: &str) -> Option<Self> {
		match value {
			"admin" => Some(Self::Admin),
			"employee" => Some(Self::Employee),
			_ => None,
		}
	}

	pub fn as_str(self) -> &'static str {
		match self {
			Self::Admin => "admin",
			Self::Employee => "employee",
		}
	}
}

#[derive(Clone, Debug, Serialize)]
pub struct AuthUser {
	pub id: Uuid,
	pub email: String,
	pub role: Role,
}

impl AuthUser {
	/// Every data handler calls this explicitly, reads included.
	pub fn require_role(&self, roles: &[Role]) -> Result<(), AppError> {
		if roles.contains(&self.role) {
			Ok(())
		} else {
			Err(AppError::Forbidden(
				ErrorCode::Forbidden,
				"Insufficient permissions".into(),
			))
		}
	}
}

/// Reads the user that `middleware::require_session` put in request extensions. A handler
/// that takes `AuthUser` but sits outside the protected router gets a 401, not a free pass.
impl<S: Send + Sync> FromRequestParts<S> for AuthUser {
	type Rejection = AppError;

	async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
		parts.extensions.get::<AuthUser>().cloned().ok_or_else(|| {
			AppError::Unauthorized(ErrorCode::NotAuthenticated, "Not authenticated".into())
		})
	}
}

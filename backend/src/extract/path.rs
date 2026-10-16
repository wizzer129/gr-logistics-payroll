use axum::{
	extract::{FromRequestParts, Path},
	http::request::Parts,
};
use uuid::Uuid;

use crate::error::AppError;

/// A `/{id}` path segment that must be a UUID. A malformed one is a 400 with the usual
/// `{ "error" }` body (plain `Path<Uuid>` replies in plain text).
pub struct IdPath(pub Uuid);

impl<S: Send + Sync> FromRequestParts<S> for IdPath {
	type Rejection = AppError;

	async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
		let Path(id) = Path::<Uuid>::from_request_parts(parts, state)
			.await
			.map_err(|_| AppError::BadRequest("Invalid id".into()))?;
		Ok(Self(id))
	}
}

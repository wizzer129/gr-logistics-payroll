use axum::{
	extract::{FromRequestParts, Query},
	http::request::Parts,
};
use serde::de::DeserializeOwned;

use crate::error::AppError;

/// `Query<T>` whose rejection (e.g. `employeeId=abc`) is a 400 with the `{ "error" }` body.
pub struct ApiQuery<T>(pub T);

impl<T, S> FromRequestParts<S> for ApiQuery<T>
where
	T: DeserializeOwned,
	S: Send + Sync,
{
	type Rejection = AppError;

	async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
		let Query(value) = Query::<T>::from_request_parts(parts, state)
			.await
			.map_err(|rejection| AppError::BadRequest(rejection.body_text()))?;
		Ok(Self(value))
	}
}

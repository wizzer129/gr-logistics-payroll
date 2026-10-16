use axum::{
	Json,
	extract::{FromRequest, Request},
};
use serde::de::DeserializeOwned;

use crate::error::AppError;

/// `Json<T>` + garde validation, with **every** failure mapped to 400 `{ "error" }`. Plain
/// `axum::Json` would return 422 for type errors and plain-text bodies.
pub struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
	T: DeserializeOwned + garde::Validate,
	T::Context: Default,
	S: Send + Sync,
{
	type Rejection = AppError;

	async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
		let Json(value) = Json::<T>::from_request(req, state)
			.await
			.map_err(|rejection| AppError::BadRequest(rejection.body_text()))?;

		value
			.validate()
			.map_err(|report| AppError::BadRequest(report.to_string()))?;

		Ok(Self(value))
	}
}

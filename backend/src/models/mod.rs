//! Request/response DTOs. Every struct uses `#[serde(rename_all = "camelCase")]` — the
//! frontend contract is camelCase keys, money as JSON numbers, dates as "YYYY-MM-DD".

pub mod auth;
pub mod dashboard;
pub mod email;
pub mod employee;
pub mod load;
pub mod money;
pub mod paycheck_log;

use serde::{Deserialize, Deserializer};

/// Optional text fields: `""` / whitespace means "not provided". The frontend's employee
/// form sends `email: ""` when the field is left blank.
pub(crate) fn blank_as_none<'de, D: Deserializer<'de>>(
	deserializer: D,
) -> Result<Option<String>, D::Error> {
	let value = Option::<String>::deserialize(deserializer)?;
	Ok(value.filter(|s| !s.trim().is_empty()))
}

/// garde rule for a language field: `en` or `es` (`email::Locale`).
pub(crate) fn locale(value: &str, _ctx: &()) -> garde::Result {
	match crate::email::Locale::parse(value) {
		Some(_) => Ok(()),
		None => Err(garde::Error::new("must be \"en\" or \"es\"")),
	}
}

pub(crate) fn optional_locale(value: &Option<String>, ctx: &()) -> garde::Result {
	value.as_deref().map_or(Ok(()), |value| locale(value, ctx))
}

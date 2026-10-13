//! Request/response DTOs. Every struct uses `#[serde(rename_all = "camelCase")]` — the
//! frontend contract is camelCase keys, money as JSON numbers, dates as "YYYY-MM-DD".

pub mod auth;
pub mod dashboard;
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

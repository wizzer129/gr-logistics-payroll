use chrono::NaiveDate;
use garde::Validate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{blank_as_none, money};

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Employee {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub dob: Option<NaiveDate>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub active: bool,
    #[serde(with = "money::number")]
    pub pay_rate_percent: Decimal,
}

/// `Serialize` (skipping absent fields) is what gets written to `audit_log.diff`.
#[derive(Debug, Deserialize, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateEmployee {
    #[garde(length(min = 1))]
    pub first_name: String,
    #[garde(length(min = 1))]
    pub last_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub dob: Option<NaiveDate>,
    #[serde(
        default,
        deserialize_with = "blank_as_none",
        skip_serializing_if = "Option::is_none"
    )]
    #[garde(email)]
    pub email: Option<String>,
    #[serde(
        default,
        deserialize_with = "blank_as_none",
        skip_serializing_if = "Option::is_none"
    )]
    #[garde(skip)]
    pub address: Option<String>,
    /// Defaults to 42% when omitted.
    #[serde(
        default,
        with = "money::number_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[garde(custom(money::pay_rate))]
    pub pay_rate_percent: Option<Decimal>,
}

/// Partial update: an omitted field keeps its current value (`COALESCE`; a field can't be
/// cleared to null this way).
#[derive(Debug, Deserialize, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEmployee {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(length(min = 1))]
    pub first_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(length(min = 1))]
    pub last_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub dob: Option<NaiveDate>,
    #[serde(
        default,
        deserialize_with = "blank_as_none",
        skip_serializing_if = "Option::is_none"
    )]
    #[garde(email)]
    pub email: Option<String>,
    #[serde(
        default,
        deserialize_with = "blank_as_none",
        skip_serializing_if = "Option::is_none"
    )]
    #[garde(skip)]
    pub address: Option<String>,
    #[serde(
        default,
        with = "money::number_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[garde(custom(money::pay_rate))]
    pub pay_rate_percent: Option<Decimal>,
}

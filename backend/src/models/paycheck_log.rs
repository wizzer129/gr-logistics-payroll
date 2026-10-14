use chrono::{DateTime, NaiveDate, Utc};
use garde::Validate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{load::LoadInput, money};

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PaycheckLog {
    pub id: Uuid,
    pub employee_id: Uuid,
    pub week_start: NaiveDate,
    pub week_end: NaiveDate,
    pub payday: NaiveDate,
    /// `draft` or `completed`.
    pub status: String,
    /// `loads` (itemized) or `lump_sum` (one amount, no loads, paid at 100%).
    pub kind: String,
    #[serde(with = "money::number")]
    pub summary_total: Decimal,
    #[serde(with = "money::number")]
    pub employee_cut: Decimal,
    /// The rate `employee_cut` was computed at, and the rate edits recompute at — never the
    /// employee's current rate, so a rate change can't re-rate history.
    #[serde(with = "money::number")]
    pub pay_rate_percent: Decimal,
}

/// GET /api/paycheck-logs row: the log plus the employee's name and who processed it when.
#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PaycheckLogListItem {
    #[serde(flatten)]
    #[sqlx(flatten)]
    pub log: PaycheckLog,
    pub first_name: String,
    pub last_name: String,
    pub created_at: DateTime<Utc>,
    /// None if the creating user was deleted (or the log predates `created_by`).
    pub created_by_email: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaycheckLogFilter {
    pub employee_id: Option<Uuid>,
    /// Logs whose week starts on or after this date.
    pub from: Option<NaiveDate>,
    /// Logs whose week ends on or before this date.
    pub to: Option<NaiveDate>,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreatePaycheckLog {
    #[garde(skip)]
    pub employee_id: Uuid,
    #[garde(skip)]
    pub week_start: NaiveDate,
    #[garde(skip)]
    pub week_end: NaiveDate,
    #[garde(skip)]
    pub payday: NaiveDate,
    /// Empty for a lump sum. Send either `loads` or `lumpSum`, never both.
    #[serde(default)]
    #[garde(dive)]
    pub loads: Vec<LoadInput>,
    /// A lump-sum payment instead of loads: paid in full, no cut taken.
    #[serde(
        default,
        with = "money::number_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[garde(custom(money::lump_sum))]
    pub lump_sum: Option<Decimal>,
    /// `true` saves the log as `completed` right away (in the same transaction as the
    /// create); omitted or `false` saves a `draft`.
    #[serde(default)]
    #[garde(skip)]
    pub complete: bool,
}

/// Editing replaces the week range, payday and the full set of loads (or the lump sum). The
/// employee and the log's kind are immutable, so there's no `employee_id` field (a sent one
/// is ignored) and a lump-sum log can't be edited into a loads log or back.
#[derive(Debug, Deserialize, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePaycheckLog {
    #[garde(skip)]
    pub week_start: NaiveDate,
    #[garde(skip)]
    pub week_end: NaiveDate,
    #[garde(skip)]
    pub payday: NaiveDate,
    /// Empty for a lump sum. Send either `loads` or `lumpSum`, never both.
    #[serde(default)]
    #[garde(dive)]
    pub loads: Vec<LoadInput>,
    /// A lump-sum payment instead of loads: paid in full, no cut taken.
    #[serde(
        default,
        with = "money::number_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[garde(custom(money::lump_sum))]
    pub lump_sum: Option<Decimal>,
    /// `true` also marks the log `completed`, in the same transaction as the edit. Omitted
    /// or `false` leaves the status as it is (a completed log stays completed).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[garde(skip)]
    pub complete: bool,
}

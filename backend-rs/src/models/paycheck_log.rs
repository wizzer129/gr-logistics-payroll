use chrono::NaiveDate;
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
    #[serde(with = "money::number")]
    pub summary_total: Decimal,
    #[serde(with = "money::number")]
    pub employee_cut: Decimal,
    /// The rate `employee_cut` was computed at, and the rate edits recompute at. `null` only
    /// for logs the Bun backend created after migration 0007 (edits then use the
    /// employee's current rate and store it).
    #[serde(with = "money::number_option")]
    pub pay_rate_percent: Option<Decimal>,
}

/// GET /api/paycheck-logs row: the log plus the employee's name.
#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PaycheckLogListItem {
    #[serde(flatten)]
    #[sqlx(flatten)]
    pub log: PaycheckLog,
    pub first_name: String,
    pub last_name: String,
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
    #[garde(length(min = 1), dive)]
    pub loads: Vec<LoadInput>,
}

/// Editing replaces the week range, payday and the full set of loads. The employee is
/// immutable, so there's no `employee_id` field (a sent one is ignored).
#[derive(Debug, Deserialize, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePaycheckLog {
    #[garde(skip)]
    pub week_start: NaiveDate,
    #[garde(skip)]
    pub week_end: NaiveDate,
    #[garde(skip)]
    pub payday: NaiveDate,
    #[garde(length(min = 1), dive)]
    pub loads: Vec<LoadInput>,
}

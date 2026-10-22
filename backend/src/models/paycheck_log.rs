use chrono::{DateTime, NaiveDate, Utc};
use garde::Validate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{load::LoadInput, money};

/// `employee_paycheck_logs.status` (a text column with a CHECK). Serialized as the same
/// strings the frontend already uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum PaycheckStatus {
	Draft,
	/// Totals are a permanent snapshot from here on (see `handlers::paycheck_logs::complete`).
	Completed,
}

/// `employee_paycheck_logs.kind` (migration 0009). Never changes after the log is created.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum PaycheckKind {
	/// Itemized loads, cut at the log's rate.
	Loads,
	/// One amount, no loads, paid in full (recorded at a 100% rate).
	LumpSum,
}

impl PaycheckKind {
	/// The stored (and serialized) name.
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Loads => "loads",
			Self::LumpSum => "lump_sum",
		}
	}
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PaycheckLog {
	pub id: Uuid,
	pub employee_id: Uuid,
	pub week_start: NaiveDate,
	pub week_end: NaiveDate,
	pub payday: NaiveDate,
	pub status: PaycheckStatus,
	pub kind: PaycheckKind,
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
	/// Logs paid on or after this date (Payroll History's filter).
	pub payday_from: Option<NaiveDate>,
	/// Logs paid on or before this date.
	pub payday_to: Option<NaiveDate>,
	/// `draft` or `completed`; both if absent.
	pub status: Option<PaycheckStatus>,
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

#[cfg(test)]
mod tests {
	use super::*;

	/// The API and the database both use these exact strings.
	#[test]
	fn status_and_kind_serialize_as_their_stored_names() {
		assert_eq!(
			serde_json::to_value(PaycheckStatus::Completed).unwrap(),
			"completed"
		);
		assert_eq!(
			serde_json::to_value(PaycheckStatus::Draft).unwrap(),
			"draft"
		);
		for kind in [PaycheckKind::Loads, PaycheckKind::LumpSum] {
			assert_eq!(serde_json::to_value(kind).unwrap(), kind.as_str());
		}
	}
}

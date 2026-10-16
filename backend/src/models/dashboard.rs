use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::money;

#[derive(Debug, Deserialize)]
pub struct YearQuery {
	/// Defaults to the current year; must be 2000–2100.
	pub year: Option<i32>,
}

/// Totals cover **completed** logs whose week starts in `year`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSummary {
	pub year: i32,
	/// Sum of employee cuts.
	#[serde(with = "money::number")]
	pub ytd_total: Decimal,
	/// Sum of load totals.
	#[serde(with = "money::number")]
	pub ytd_gross: Decimal,
	pub paycheck_count: i64,
	pub total_loads: i64,
	pub by_employee: Vec<EmployeeTotal>,
	pub by_month: Vec<MonthTotal>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeTotal {
	pub employee_id: Uuid,
	pub first_name: String,
	pub last_name: String,
	#[serde(with = "money::number")]
	pub total: Decimal,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct MonthTotal {
	/// "YYYY-MM"
	pub period: String,
	pub employee_id: Uuid,
	pub first_name: String,
	pub last_name: String,
	#[serde(with = "money::number")]
	pub total: Decimal,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeName {
	pub id: Uuid,
	pub first_name: String,
	pub last_name: String,
}

/// Both totals count completed logs only, bucketed by `week_start`. `month_total` is the
/// current calendar month, whatever `year` was asked for.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeSummary {
	pub year: i32,
	pub employee: EmployeeName,
	#[serde(with = "money::number")]
	pub ytd_total: Decimal,
	#[serde(with = "money::number")]
	pub month_total: Decimal,
}

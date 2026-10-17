//! What the monthly report is built from, as `services::monthly_report::source` reads it.
//! `pdf::monthly_report::document` turns it into the PDF's and email's text.

use chrono::NaiveDate;
use rust_decimal::Decimal;

/// One employee's completed paychecks in a month (see `services::monthly_report`).
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct EmployeeMonth {
	pub name: String,
	pub paychecks: i64,
	pub loads: i64,
	pub gross: Decimal,
	pub paid: Decimal,
}

pub struct MonthlyReportSource {
	/// First day of the reported month.
	pub month: NaiveDate,
	pub employees: Vec<EmployeeMonth>,
	/// The month before, for the comparison.
	pub previous: Vec<EmployeeMonth>,
	/// Draft paychecks with a payday in the month: left out of every number, but noted.
	pub drafts: i64,
}

//! What a pay stub is built from: one paycheck log as `services::paystubs::source` reads it.
//! `pdf::paystub::document` turns it into the PDF's and email's text.

use chrono::NaiveDate;
use rust_decimal::Decimal;

pub struct PaystubLoad {
	pub store_name: String,
	pub load_date: NaiveDate,
	pub pay: Decimal,
	pub extra_pay: Decimal,
}

/// What the database holds for one log (see `services::paystubs`).
pub struct PaystubSource {
	pub employee_name: String,
	pub week_start: NaiveDate,
	pub week_end: NaiveDate,
	pub payday: NaiveDate,
	pub lump_sum: bool,
	pub summary_total: Decimal,
	pub pay_rate_percent: Decimal,
	pub loads: Vec<PaystubLoad>,
}

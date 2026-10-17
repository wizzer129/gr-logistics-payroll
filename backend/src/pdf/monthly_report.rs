//! A month of paychecks → the monthly report's text and numbers, translated and formatted for
//! one locale (EMAIL-SPEC.md §8). The same document fills the PDF and the email body.

use chrono::{Months, NaiveDate};
use rust_decimal::Decimal;
use serde::Serialize;
use serde_json::Value;

use crate::{domain::format, locale::Locale};

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

struct Totals {
	paychecks: i64,
	loads: i64,
	gross: Decimal,
	paid: Decimal,
}

fn totals(employees: &[EmployeeMonth]) -> Totals {
	Totals {
		paychecks: employees.iter().map(|e| e.paychecks).sum(),
		loads: employees.iter().map(|e| e.loads).sum(),
		gross: employees.iter().map(|e| e.gross).sum(),
		paid: employees.iter().map(|e| e.paid).sum(),
	}
}

#[derive(Debug, Serialize)]
pub struct Stat {
	pub value: String,
	/// `+12%` against the previous month; `None` when that month had nothing.
	pub change: Option<String>,
}

impl Stat {
	fn money(current: Decimal, previous: Decimal) -> Self {
		Self {
			value: format::money(current),
			change: format::change(current, previous),
		}
	}

	fn count(current: i64, previous: i64) -> Self {
		Self {
			value: current.to_string(),
			change: format::change(current.into(), previous.into()),
		}
	}
}

#[derive(Debug, Serialize)]
pub struct EmployeeRow {
	pub name: String,
	pub paychecks: String,
	pub loads: String,
	pub gross: String,
	pub paid: String,
}

impl EmployeeRow {
	fn new(name: String, month: &Totals) -> Self {
		Self {
			name,
			paychecks: month.paychecks.to_string(),
			loads: month.loads.to_string(),
			gross: format::money(month.gross),
			paid: format::money(month.paid),
		}
	}
}

#[derive(Debug, Serialize)]
pub struct MonthlyReportDocument {
	pub month: String,
	/// `vs. September 2026`, shown after each change.
	pub vs_previous: String,
	pub paid: Stat,
	pub gross: Stat,
	pub paychecks: Stat,
	pub loads: Stat,
	pub drafts_note: Option<String>,
	pub employees: Vec<EmployeeRow>,
	pub total: EmployeeRow,
	/// The `monthly_report_pdf` section of the locale file.
	pub labels: Value,
}

/// `labels` is the locale file's `monthly_report_pdf` section.
pub fn document(
	source: &MonthlyReportSource,
	locale: Locale,
	labels: &Value,
) -> MonthlyReportDocument {
	let label = |key: &str| labels[key].as_str().unwrap_or_default().to_owned();
	let month = format::month_year(source.month, locale);
	let current = totals(&source.employees);
	let previous = totals(&source.previous);

	let drafts_note = (source.drafts > 0).then(|| {
		let key = if source.drafts == 1 {
			"drafts_one"
		} else {
			"drafts_other"
		};
		label(key)
			.replace("{count}", &source.drafts.to_string())
			.replace("{month}", &month)
	});

	MonthlyReportDocument {
		vs_previous: label("vs_previous").replace(
			"{month}",
			&format::month_year(source.month - Months::new(1), locale),
		),
		paid: Stat::money(current.paid, previous.paid),
		gross: Stat::money(current.gross, previous.gross),
		paychecks: Stat::count(current.paychecks, previous.paychecks),
		loads: Stat::count(current.loads, previous.loads),
		drafts_note,
		employees: source
			.employees
			.iter()
			.map(|e| EmployeeRow::new(e.name.clone(), &totals(std::slice::from_ref(e))))
			.collect(),
		total: EmployeeRow::new(label("total"), &current),
		month,
		labels: labels.clone(),
	}
}

#[cfg(test)]
mod tests {
	use rust_decimal_macros::dec;
	use serde_json::json;

	use super::*;

	fn employee(name: &str, paychecks: i64, gross: Decimal, paid: Decimal) -> EmployeeMonth {
		EmployeeMonth {
			name: name.into(),
			paychecks,
			loads: paychecks * 3,
			gross,
			paid,
		}
	}

	fn labels() -> Value {
		json!({
			"total": "Total", "vs_previous": "vs. {month}",
			"drafts_one": "{count} draft in {month}", "drafts_other": "{count} drafts in {month}",
		})
	}

	fn source(drafts: i64) -> MonthlyReportSource {
		MonthlyReportSource {
			month: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
			employees: vec![
				employee("Ana Diaz", 4, dec!(4000), dec!(1680)),
				employee("Luis Mora", 2, dec!(1000.50), dec!(420)),
			],
			previous: vec![employee("Ana Diaz", 4, dec!(4000), dec!(1500))],
			drafts,
		}
	}

	#[test]
	fn totals_and_changes_against_the_previous_month() {
		let doc = document(&source(0), Locale::En, &labels());
		assert_eq!(doc.month, "September 2026");
		assert_eq!(doc.vs_previous, "vs. August 2026");
		assert_eq!(doc.paid.value, "$2,100.00");
		assert_eq!(doc.paid.change.as_deref(), Some("+40%"));
		assert_eq!(doc.gross.value, "$5,000.50");
		assert_eq!(doc.paychecks.value, "6");
		assert_eq!(doc.paychecks.change.as_deref(), Some("+50%"));
		assert_eq!(doc.loads.value, "18");
		assert_eq!(doc.employees.len(), 2);
		assert_eq!(doc.employees[1].gross, "$1,000.50");
		assert_eq!(doc.total.name, "Total");
		assert_eq!(doc.total.paid, "$2,100.00");
		assert_eq!(doc.drafts_note, None);
	}

	#[test]
	fn no_previous_month_means_no_change() {
		let mut source = source(0);
		source.previous.clear();
		let doc = document(&source, Locale::Es, &labels());
		assert_eq!(doc.month, "septiembre de 2026");
		assert_eq!(doc.paid.change, None);
	}

	#[test]
	fn drafts_are_noted_with_the_right_plural() {
		let one = document(&source(1), Locale::En, &labels());
		assert_eq!(
			one.drafts_note.as_deref(),
			Some("1 draft in September 2026")
		);
		let three = document(&source(3), Locale::En, &labels());
		assert_eq!(
			three.drafts_note.as_deref(),
			Some("3 drafts in September 2026")
		);
	}

	#[test]
	fn renders_a_pdf() {
		let mut source = source(2);
		// Enough rows to spill onto a second page.
		for i in 0..60 {
			source
				.employees
				.push(employee(&format!("Employee {i}"), 1, dec!(100), dec!(42)));
		}
		let strings: Value =
			serde_json::from_str(include_str!("../../locales/email/en.json")).unwrap();
		let doc = document(&source, Locale::En, &strings["monthly_report_pdf"]);
		let pdf = crate::pdf::monthly_report(&doc).unwrap();
		assert!(pdf.starts_with(b"%PDF"));
	}
}

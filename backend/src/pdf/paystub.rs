//! A paycheck log → the pay stub's text and numbers, translated and formatted for one
//! locale. The same document fills the PDF and the email body. Amounts follow the screen:
//! the cut is computed at the log's own rate and rounded half-up to the dollar
//! (`domain::payment_calc`); a lump sum is paid in full.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::Serialize;
use serde_json::Value;

use crate::{
	domain::{
		format,
		payment_calc::{EmployeeCut, calculate_employee_cut},
	},
	locale::Locale,
};

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

#[derive(Debug, Serialize)]
pub struct LoadRow {
	pub date: String,
	pub store: String,
	pub pay: String,
	/// `None` shows a dash, like the screen.
	pub extra: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PaystubDocument {
	pub company: String,
	pub employee_name: String,
	pub week: String,
	pub payday: String,
	pub lump_sum: bool,
	pub loads: Vec<LoadRow>,
	pub extra_total: String,
	pub summary_total: String,
	pub employee_cut_label: String,
	pub employee_cut: String,
	pub rounded_pay: String,
	/// The `paystub_pdf` section of the locale file.
	pub labels: Value,
}

/// `labels` is the locale file's `paystub_pdf` section.
pub fn document(source: &PaystubSource, locale: Locale, labels: &Value) -> PaystubDocument {
	let (rate, cut) = if source.lump_sum {
		let amount = source.summary_total;
		(
			Decimal::ONE_HUNDRED,
			EmployeeCut {
				employee_cut: amount,
				rounded_pay: amount,
			},
		)
	} else {
		(
			source.pay_rate_percent,
			calculate_employee_cut(source.summary_total, source.pay_rate_percent),
		)
	};

	let mut loads: Vec<_> = source.loads.iter().collect();
	loads.sort_by_key(|load| load.load_date);
	let extra_total: Decimal = loads.iter().map(|load| load.extra_pay).sum();

	let label = |key: &str| labels[key].as_str().unwrap_or_default().to_owned();
	PaystubDocument {
		company: label("company"),
		employee_name: source.employee_name.clone(),
		week: format::week_range(source.week_start, source.week_end, locale),
		payday: format::full_date(source.payday, locale),
		lump_sum: source.lump_sum,
		loads: loads
			.into_iter()
			.map(|load| LoadRow {
				date: format::weekday_short_date(load.load_date, locale),
				store: load.store_name.clone(),
				pay: format::money(load.pay),
				extra: (!load.extra_pay.is_zero()).then(|| format::money(load.extra_pay)),
			})
			.collect(),
		extra_total: format::money(extra_total),
		summary_total: format::money(source.summary_total),
		employee_cut_label: label("employee_cut").replace("{rate}", &format::rate(rate)),
		employee_cut: format::money(cut.employee_cut),
		rounded_pay: format::money(cut.rounded_pay),
		labels: labels.clone(),
	}
}

#[cfg(test)]
mod tests {
	use rust_decimal_macros::dec;
	use serde_json::json;

	use super::*;

	fn date(d: u32) -> NaiveDate {
		NaiveDate::from_ymd_opt(2026, 10, d).unwrap()
	}

	fn labels() -> Value {
		json!({ "company": "GR Logistics", "employee_cut": "Employee Cut ({rate}%)" })
	}

	fn source(lump_sum: bool) -> PaystubSource {
		PaystubSource {
			employee_name: "Ana Diaz".into(),
			week_start: date(4),
			week_end: date(10),
			payday: date(16),
			lump_sum,
			summary_total: dec!(1050.50),
			pay_rate_percent: dec!(42),
			loads: if lump_sum {
				vec![]
			} else {
				vec![
					PaystubLoad {
						store_name: "Store B".into(),
						load_date: date(6),
						pay: dec!(500),
						extra_pay: dec!(50.50),
					},
					PaystubLoad {
						store_name: "Store A".into(),
						load_date: date(5),
						pay: dec!(500),
						extra_pay: dec!(0),
					},
				]
			},
		}
	}

	#[test]
	fn loads_are_cut_at_the_logs_rate_and_rounded_half_up() {
		let doc = document(&source(false), Locale::En, &labels());
		// 1050.50 × 42% = 441.21 → $441.
		assert_eq!(doc.summary_total, "$1,050.50");
		assert_eq!(doc.employee_cut, "$441.21");
		assert_eq!(doc.rounded_pay, "$441.00");
		assert_eq!(doc.employee_cut_label, "Employee Cut (42%)");
		assert_eq!(doc.extra_total, "$50.50");
		assert_eq!(doc.week, "Oct 4 – Oct 10, 2026");
		assert_eq!(doc.payday, "Oct 16, 2026");

		assert_eq!(doc.loads[0].store, "Store A", "sorted by date");
		assert_eq!(doc.loads[0].date, "Mon, 10/5");
		assert_eq!(doc.loads[0].extra, None, "no extra shows a dash");
		assert_eq!(doc.loads[1].extra.as_deref(), Some("$50.50"));
	}

	#[test]
	fn a_lump_sum_is_paid_in_full_with_cents() {
		let doc = document(&source(true), Locale::Es, &labels());
		assert_eq!(doc.employee_cut, "$1,050.50");
		assert_eq!(doc.rounded_pay, "$1,050.50", "no rounding for a lump sum");
		assert_eq!(doc.employee_cut_label, "Employee Cut (100%)");
		assert_eq!(doc.payday, "16 oct 2026");
	}

	#[test]
	fn renders_a_pdf() {
		let mut source = source(false);
		// Enough loads to take the two-column layout too.
		for day in 4..14 {
			source.loads.push(PaystubLoad {
				store_name: format!("Store {day}"),
				load_date: NaiveDate::from_ymd_opt(2026, 10, day).unwrap(),
				pay: dec!(100),
				extra_pay: dec!(0),
			});
		}
		let labels = json!({
			"company": "GR Logistics", "title": "Pay Stub", "contractor": "Contractor",
			"owner_operator": "Owner Operator", "work_week": "Work Week", "payday": "Payday",
			"load_count": "Loads", "payment_type": "Payment Type", "lump_sum": "Lump Sum",
			"date": "Date", "load": "Load", "pay": "Payment", "extra": "Extra",
			"extra_pay_total": "Extra Pay", "summary_total": "Summary Total",
			"employee_cut": "Employee Cut ({rate}%)", "total_pay": "Total Pay", "no_loads": "No loads",
		});
		for lump_sum in [false, true] {
			source.lump_sum = lump_sum;
			let pdf = crate::pdf::paystub(&document(&source, Locale::En, &labels)).unwrap();
			assert!(pdf.starts_with(b"%PDF"), "lump_sum={lump_sum}");
		}
	}
}

//! Display formatting for emails and PDFs, matching what the frontend shows: money is always
//! `$1,234.56` (en-US, deliberately, like `frontend/src/utils/currency.ts`), and dates match
//! `Intl.DateTimeFormat` for each locale (`Oct 4, 2026` / `4 oct 2026`).

use chrono::{Datelike, NaiveDate};
use rust_decimal::{Decimal, RoundingStrategy};

use crate::locale::Locale;

const MONTHS_EN: [&str; 12] = [
	"Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const MONTHS_ES: [&str; 12] = [
	"ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sept", "oct", "nov", "dic",
];
const MONTHS_LONG_EN: [&str; 12] = [
	"January",
	"February",
	"March",
	"April",
	"May",
	"June",
	"July",
	"August",
	"September",
	"October",
	"November",
	"December",
];
const MONTHS_LONG_ES: [&str; 12] = [
	"enero",
	"febrero",
	"marzo",
	"abril",
	"mayo",
	"junio",
	"julio",
	"agosto",
	"septiembre",
	"octubre",
	"noviembre",
	"diciembre",
];
/// Monday first, as chrono's `num_days_from_monday` counts.
const WEEKDAYS_EN: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const WEEKDAYS_ES: [&str; 7] = ["lun", "mar", "mié", "jue", "vie", "sáb", "dom"];

/// `$1,234.56`. Rounds half-up to cents for display (stored amounts already have ≤ 2).
pub fn money(amount: Decimal) -> String {
	let cents = amount.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
	let negative = cents.is_sign_negative() && !cents.is_zero();
	let text = format!("{:.2}", cents.abs());
	let (whole, fraction) = text.split_once('.').unwrap_or((&text, "00"));

	let mut grouped = String::new();
	for (i, digit) in whole.chars().enumerate() {
		if i > 0 && (whole.len() - i) % 3 == 0 {
			grouped.push(',');
		}
		grouped.push(digit);
	}
	format!("{}${grouped}.{fraction}", if negative { "-" } else { "" })
}

/// A percent rate without trailing zeros: `42`, `42.5`.
pub fn rate(percent: Decimal) -> String {
	percent.normalize().to_string()
}

fn month(date: NaiveDate, locale: Locale) -> &'static str {
	let index = date.month0() as usize;
	match locale {
		Locale::En => MONTHS_EN[index],
		Locale::Es => MONTHS_ES[index],
	}
}

/// `Oct 4` / `4 oct`.
pub fn month_day(date: NaiveDate, locale: Locale) -> String {
	match locale {
		Locale::En => format!("{} {}", month(date, locale), date.day()),
		Locale::Es => format!("{} {}", date.day(), month(date, locale)),
	}
}

/// `Oct 4, 2026` / `4 oct 2026`.
pub fn full_date(date: NaiveDate, locale: Locale) -> String {
	match locale {
		Locale::En => format!("{}, {}", month_day(date, locale), date.year()),
		Locale::Es => format!("{} {}", month_day(date, locale), date.year()),
	}
}

/// `October 2026` / `octubre de 2026`, for the monthly report.
pub fn month_year(date: NaiveDate, locale: Locale) -> String {
	let index = date.month0() as usize;
	match locale {
		Locale::En => format!("{} {}", MONTHS_LONG_EN[index], date.year()),
		Locale::Es => format!("{} de {}", MONTHS_LONG_ES[index], date.year()),
	}
}

/// Change from `previous` to `current` as a whole percent: `+12%`, `-5%`, `0%`. `None` when
/// there's nothing to compare against (`previous` is zero).
pub fn change(current: Decimal, previous: Decimal) -> Option<String> {
	if previous.is_zero() {
		return None;
	}
	let percent = ((current - previous) / previous * Decimal::ONE_HUNDRED)
		.round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero);
	Some(if percent.is_sign_positive() && !percent.is_zero() {
		format!("+{percent}%")
	} else {
		format!("{}%", percent.normalize())
	})
}

/// `Oct 4 – Oct 10, 2026` / `4 oct – 10 oct 2026`, like the paystub's work week.
pub fn week_range(start: NaiveDate, end: NaiveDate, locale: Locale) -> String {
	format!("{} – {}", month_day(start, locale), full_date(end, locale))
}

/// `Sun, 10/4` / `dom, 4/10`, like the paystub's load dates.
pub fn weekday_short_date(date: NaiveDate, locale: Locale) -> String {
	let weekday = date.weekday().num_days_from_monday() as usize;
	match locale {
		Locale::En => format!("{}, {}/{}", WEEKDAYS_EN[weekday], date.month(), date.day()),
		Locale::Es => format!("{}, {}/{}", WEEKDAYS_ES[weekday], date.day(), date.month()),
	}
}

#[cfg(test)]
mod tests {
	use rust_decimal_macros::dec;

	use super::*;

	fn date(y: i32, m: u32, d: u32) -> NaiveDate {
		NaiveDate::from_ymd_opt(y, m, d).unwrap()
	}

	#[test]
	fn money_is_en_us_dollars() {
		assert_eq!(money(dec!(0)), "$0.00");
		assert_eq!(money(dec!(5)), "$5.00");
		assert_eq!(money(dec!(420.5)), "$420.50");
		assert_eq!(money(dec!(1234.56)), "$1,234.56");
		assert_eq!(money(dec!(1234567.891)), "$1,234,567.89");
		assert_eq!(money(dec!(-12.3)), "-$12.30");
	}

	#[test]
	fn rates_drop_trailing_zeros() {
		assert_eq!(rate(dec!(42.00)), "42");
		assert_eq!(rate(dec!(42.50)), "42.5");
		assert_eq!(rate(dec!(100)), "100");
	}

	/// Same strings `Intl.DateTimeFormat` gives the frontend for these dates.
	#[test]
	fn dates_match_the_frontend() {
		let d = date(2026, 10, 4);
		assert_eq!(month_day(d, Locale::En), "Oct 4");
		assert_eq!(month_day(d, Locale::Es), "4 oct");
		assert_eq!(full_date(d, Locale::En), "Oct 4, 2026");
		assert_eq!(full_date(d, Locale::Es), "4 oct 2026");
		assert_eq!(weekday_short_date(d, Locale::En), "Sun, 10/4");
		assert_eq!(weekday_short_date(d, Locale::Es), "dom, 4/10");
		assert_eq!(
			week_range(d, date(2026, 10, 10), Locale::En),
			"Oct 4 – Oct 10, 2026"
		);
		assert_eq!(full_date(date(2026, 9, 1), Locale::Es), "1 sept 2026");
		assert_eq!(month_year(date(2026, 9, 1), Locale::En), "September 2026");
		assert_eq!(
			month_year(date(2026, 9, 1), Locale::Es),
			"septiembre de 2026"
		);
	}

	#[test]
	fn changes_are_whole_signed_percents() {
		assert_eq!(change(dec!(112), dec!(100)).as_deref(), Some("+12%"));
		assert_eq!(change(dec!(95), dec!(100)).as_deref(), Some("-5%"));
		assert_eq!(change(dec!(100), dec!(100)).as_deref(), Some("0%"));
		assert_eq!(
			change(dec!(100.5), dec!(100)).as_deref(),
			Some("+1%"),
			"half-up"
		);
		assert_eq!(change(dec!(100), dec!(0)), None);
	}
}

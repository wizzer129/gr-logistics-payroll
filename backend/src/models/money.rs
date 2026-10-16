//! Money and percent fields: `Decimal` end to end, never `f64`.
//!
//! On the wire they're JSON **numbers** (vue-i18n's `n()` throws on strings). With
//! serde_json's `arbitrary_precision`, `number` reads the number's exact text, so `120.1`
//! becomes `120.1`, never `120.09999…`. Use `#[serde(with = "money::number")]` on every
//! `Decimal` field, and `number_option` with `#[serde(default)]` on optional ones.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Strict: only a JSON number is accepted (`"12.5"` is a 400, as Zod's `z.number()` was).
/// rust_decimal's own `arbitrary_precision` deserializer also takes strings, so only its
/// serializer is reused.
pub mod number {
	use rust_decimal::Decimal;
	use serde::{Deserialize, Deserializer, Serializer};

	pub fn serialize<S: Serializer>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error> {
		rust_decimal::serde::arbitrary_precision::serialize(value, serializer)
	}

	pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Decimal, D::Error> {
		let number = serde_json::Number::deserialize(deserializer)?;
		super::parse(&number)
	}
}

/// `Option` form of [`number`]; pair with `#[serde(default)]` so a missing field is `None`.
pub mod number_option {
	use rust_decimal::Decimal;
	use serde::{Deserialize, Deserializer, Serializer};

	pub fn serialize<S: Serializer>(
		value: &Option<Decimal>,
		serializer: S,
	) -> Result<S::Ok, S::Error> {
		rust_decimal::serde::arbitrary_precision_option::serialize(value, serializer)
	}

	pub fn deserialize<'de, D: Deserializer<'de>>(
		deserializer: D,
	) -> Result<Option<Decimal>, D::Error> {
		Option::<serde_json::Number>::deserialize(deserializer)?
			.map(|number| super::parse(&number))
			.transpose()
	}
}

/// With serde_json's `arbitrary_precision`, a `Number` keeps the exact text it was sent as.
fn parse<E: serde::de::Error>(number: &serde_json::Number) -> Result<Decimal, E> {
	let text = number.to_string();
	Decimal::from_str_exact(&text)
		.or_else(|_| Decimal::from_scientific(&text))
		.map_err(|_| E::custom(format!("invalid amount: {text}")))
}

/// Largest value a `numeric(10, 2)` column holds.
const MAX_AMOUNT: Decimal = dec!(99999999.99);

fn check_cents(value: &Decimal) -> garde::Result {
	if value.normalize().scale() > 2 {
		return Err(garde::Error::new("must have at most 2 decimal places"));
	}
	Ok(())
}

/// garde rule for `numeric(10, 2)` amounts (pay, extra pay): 0 ≤ value, at most 2 decimal places.
pub fn amount(value: &Decimal, _ctx: &()) -> garde::Result {
	if value.is_sign_negative() && !value.is_zero() {
		return Err(garde::Error::new("must not be negative"));
	}
	if *value > MAX_AMOUNT {
		return Err(garde::Error::new("is too large"));
	}
	check_cents(value)
}

/// garde rule for a lump-sum payment: like [`amount`], but it must be more than 0.
pub fn lump_sum(value: &Option<Decimal>, ctx: &()) -> garde::Result {
	let Some(value) = value else { return Ok(()) };
	if *value <= Decimal::ZERO {
		return Err(garde::Error::new("must be greater than 0"));
	}
	amount(value, ctx)
}

/// garde rule for `pay_rate_percent` (`numeric(5, 2)`): 0 < value ≤ 100.
pub fn pay_rate(value: &Option<Decimal>, _ctx: &()) -> garde::Result {
	let Some(value) = value else { return Ok(()) };
	if *value <= Decimal::ZERO || *value > Decimal::ONE_HUNDRED {
		return Err(garde::Error::new("must be greater than 0 and at most 100"));
	}
	check_cents(value)
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde::{Deserialize, Serialize};

	#[derive(Deserialize, Serialize)]
	struct Wrapper {
		#[serde(with = "number")]
		value: Decimal,
	}

	fn parse(json: &str) -> Decimal {
		serde_json::from_str::<Wrapper>(json).unwrap().value
	}

	#[test]
	fn parses_json_numbers_exactly() {
		assert_eq!(parse(r#"{"value":120.1}"#), dec!(120.10));
		assert_eq!(parse(r#"{"value":0.1}"#).to_string(), "0.1");
		assert_eq!(parse(r#"{"value":1001}"#), dec!(1001));
	}

	#[test]
	fn serializes_as_a_json_number() {
		let json = serde_json::to_string(&Wrapper { value: dec!(42.00) }).unwrap();
		assert_eq!(json, r#"{"value":42.00}"#);
	}

	#[test]
	fn rejects_non_numbers() {
		for json in [
			r#"{"value":"12.5"}"#,
			r#"{"value":null}"#,
			r#"{"value":true}"#,
		] {
			assert!(serde_json::from_str::<Wrapper>(json).is_err(), "{json}");
		}
	}

	#[test]
	fn option_accepts_missing_and_null() {
		#[derive(Deserialize)]
		struct Optional {
			#[serde(default, with = "number_option")]
			value: Option<Decimal>,
		}
		let parse = |json| serde_json::from_str::<Optional>(json).map(|o| o.value);
		assert_eq!(parse("{}").unwrap(), None);
		assert_eq!(parse(r#"{"value":null}"#).unwrap(), None);
		assert_eq!(parse(r#"{"value":40.1}"#).unwrap(), Some(dec!(40.10)));
		assert!(parse(r#"{"value":"40"}"#).is_err());
	}

	#[test]
	fn amount_rules() {
		assert!(amount(&dec!(0), &()).is_ok());
		assert!(amount(&dec!(120.10), &()).is_ok());
		assert!(amount(&dec!(1.500), &()).is_ok(), "trailing zeros are fine");
		assert!(amount(&dec!(1.005), &()).is_err());
		assert!(amount(&dec!(-0.01), &()).is_err());
		assert!(amount(&dec!(100000000), &()).is_err());
	}

	#[test]
	fn pay_rate_rules() {
		assert!(pay_rate(&None, &()).is_ok());
		assert!(pay_rate(&Some(dec!(42)), &()).is_ok());
		assert!(pay_rate(&Some(dec!(100)), &()).is_ok());
		assert!(pay_rate(&Some(dec!(0)), &()).is_err());
		assert!(pay_rate(&Some(dec!(100.01)), &()).is_err());
		assert!(pay_rate(&Some(dec!(40.555)), &()).is_err());
	}
}

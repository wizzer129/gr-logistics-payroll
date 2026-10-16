use chrono::NaiveDate;
use garde::Validate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::money;
use crate::domain::payment_calc::LoadAmounts;

/// One load as the frontend sends it. Unknown keys (the edit dialog sends back each load's
/// `id`) are ignored. `Serialize` is for the audit diff.
#[derive(Debug, Deserialize, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct LoadInput {
	/// Free text; resolved to a `stores` row case-insensitively.
	#[garde(custom(not_blank))]
	pub store_name: String,
	#[garde(skip)]
	pub load_date: NaiveDate,
	#[serde(with = "money::number")]
	#[garde(custom(money::amount))]
	pub pay: Decimal,
	/// Optional on input; omitted means 0.
	#[serde(default, with = "money::number")]
	#[garde(custom(money::amount))]
	pub extra_pay: Decimal,
}

impl LoadInput {
	pub fn amounts(&self) -> LoadAmounts {
		LoadAmounts {
			pay: self.pay,
			extra_pay: self.extra_pay,
		}
	}
}

fn not_blank(value: &str, _ctx: &()) -> garde::Result {
	if value.trim().is_empty() {
		return Err(garde::Error::new("must not be blank"));
	}
	Ok(())
}

/// GET /api/paycheck-logs/{id}/loads row.
#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Load {
	pub id: Uuid,
	pub store_name: String,
	pub load_date: NaiveDate,
	#[serde(with = "money::number")]
	pub pay: Decimal,
	#[serde(with = "money::number")]
	pub extra_pay: Decimal,
}

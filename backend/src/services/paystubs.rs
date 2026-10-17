//! What a pay stub is built from (EMAIL-SPEC.md §8): one paycheck log, its employee's name and
//! its loads.

use sqlx::PgPool;
use uuid::Uuid;

use crate::{
	pdf::paystub::{PaystubLoad, PaystubSource},
	services::paycheck_logs,
};

/// `draft` or `completed`, or `None` if there's no such log.
pub async fn status(pool: &PgPool, log_id: Uuid) -> Result<Option<String>, sqlx::Error> {
	sqlx::query_scalar("SELECT status FROM employee_paycheck_logs WHERE id = $1")
		.bind(log_id)
		.fetch_optional(pool)
		.await
}

pub async fn source(pool: &PgPool, log_id: Uuid) -> Result<Option<PaystubSource>, sqlx::Error> {
	type Row = (
		String,
		String,
		chrono::NaiveDate,
		chrono::NaiveDate,
		chrono::NaiveDate,
		String,
		rust_decimal::Decimal,
		rust_decimal::Decimal,
	);
	let row: Option<Row> = sqlx::query_as(
		"SELECT e.first_name, e.last_name, pl.week_start, pl.week_end, pl.payday, pl.kind,
			COALESCE(pl.summary_total, 0), pl.pay_rate_percent
		 FROM employee_paycheck_logs pl
		 JOIN employees e ON e.id = pl.employee_id
		 WHERE pl.id = $1",
	)
	.bind(log_id)
	.fetch_optional(pool)
	.await?;
	let Some((first, last, week_start, week_end, payday, kind, summary_total, pay_rate_percent)) =
		row
	else {
		return Ok(None);
	};

	let loads = paycheck_logs::loads(pool, log_id)
		.await?
		.into_iter()
		.map(|load| PaystubLoad {
			store_name: load.store_name,
			load_date: load.load_date,
			pay: load.pay,
			extra_pay: load.extra_pay,
		})
		.collect();

	Ok(Some(PaystubSource {
		employee_name: format!("{first} {last}"),
		week_start,
		week_end,
		payday,
		lump_sum: kind == "lump_sum",
		summary_total,
		pay_rate_percent,
		loads,
	}))
}

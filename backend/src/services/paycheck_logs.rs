//! Paycheck log + load SQL. Writes take the caller's transaction; the handler composes
//! log, loads, stores and audit into one atomic unit.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::{
	models::{
		load::{Load, LoadInput},
		paycheck_log::{
			PaycheckKind, PaycheckLog, PaycheckLogFilter, PaycheckLogListItem, PaycheckStatus,
		},
	},
	services::stores::resolve_store_ids,
};

/// The total columns are nullable in the schema but always set by this app; COALESCE keeps
/// the response a number either way.
macro_rules! log_columns {
	() => {
		"id, employee_id, week_start, week_end, payday, status, kind,
         COALESCE(summary_total, 0) AS summary_total, COALESCE(employee_cut, 0) AS employee_cut,
         pay_rate_percent"
	};
}

pub struct LogFields {
	/// Only written on insert (a log's kind never changes).
	pub kind: PaycheckKind,
	pub week_start: NaiveDate,
	pub week_end: NaiveDate,
	pub payday: NaiveDate,
	pub summary_total: Decimal,
	pub employee_cut: Decimal,
	pub pay_rate_percent: Decimal,
}

pub async fn list(
	pool: &PgPool,
	filter: &PaycheckLogFilter,
) -> Result<Vec<PaycheckLogListItem>, sqlx::Error> {
	sqlx::query_as(
		"SELECT pl.id, pl.employee_id, pl.week_start, pl.week_end, pl.payday, pl.status, pl.kind,
            COALESCE(pl.summary_total, 0) AS summary_total,
            COALESCE(pl.employee_cut, 0) AS employee_cut,
            pl.pay_rate_percent,
            e.first_name, e.last_name, pl.created_at, u.email AS created_by_email
         FROM employee_paycheck_logs pl
         JOIN employees e ON e.id = pl.employee_id
         LEFT JOIN users u ON u.id = pl.created_by
         WHERE ($1::uuid IS NULL OR pl.employee_id = $1)
         AND ($2::date IS NULL OR pl.week_start >= $2)
         AND ($3::date IS NULL OR pl.week_end <= $3)
         AND ($4::date IS NULL OR pl.payday >= $4)
         AND ($5::date IS NULL OR pl.payday <= $5)
         AND ($6::text IS NULL OR pl.status = $6)
         ORDER BY pl.week_start DESC",
	)
	.bind(filter.employee_id)
	.bind(filter.from)
	.bind(filter.to)
	.bind(filter.payday_from)
	.bind(filter.payday_to)
	.bind(filter.status)
	.fetch_all(pool)
	.await
}

pub async fn employee_pay_rate(
	conn: &mut PgConnection,
	employee_id: Uuid,
) -> Result<Option<Decimal>, sqlx::Error> {
	sqlx::query_scalar("SELECT pay_rate_percent FROM employees WHERE id = $1")
		.bind(employee_id)
		.fetch_optional(conn)
		.await
}

/// Locks the log row for the rest of the transaction (concurrent edits serialize) and
/// returns its employee, the log's own pay rate (which edits recompute at) and its kind.
pub async fn lock_for_update(
	conn: &mut PgConnection,
	id: Uuid,
) -> Result<Option<(Uuid, Decimal, PaycheckKind)>, sqlx::Error> {
	sqlx::query_as(
		"SELECT employee_id, pay_rate_percent, kind
         FROM employee_paycheck_logs
         WHERE id = $1
         FOR UPDATE",
	)
	.bind(id)
	.fetch_optional(conn)
	.await
}

pub async fn insert(
	conn: &mut PgConnection,
	employee_id: Uuid,
	fields: &LogFields,
	status: PaycheckStatus,
	user_id: Uuid,
) -> Result<PaycheckLog, sqlx::Error> {
	sqlx::query_as(concat!(
		"INSERT INTO employee_paycheck_logs
            (employee_id, week_start, week_end, payday, status, summary_total, employee_cut,
             pay_rate_percent, created_by, last_edited_by, kind)
         VALUES ($1, $2, $3, $4, $10, $5, $6, $8, $7, $7, $9)
         RETURNING ",
		log_columns!()
	))
	.bind(employee_id)
	.bind(fields.week_start)
	.bind(fields.week_end)
	.bind(fields.payday)
	.bind(fields.summary_total)
	.bind(fields.employee_cut)
	.bind(user_id)
	.bind(fields.pay_rate_percent)
	.bind(fields.kind)
	.bind(status)
	.fetch_one(conn)
	.await
}

/// `complete` also flips the status to `completed`; otherwise the status is left alone.
pub async fn update(
	conn: &mut PgConnection,
	id: Uuid,
	fields: &LogFields,
	complete: bool,
	user_id: Uuid,
) -> Result<PaycheckLog, sqlx::Error> {
	sqlx::query_as(concat!(
		"UPDATE employee_paycheck_logs SET
            week_start = $2, week_end = $3, payday = $4,
            summary_total = $5, employee_cut = $6, pay_rate_percent = $8,
            status = CASE WHEN $9 THEN 'completed' ELSE status END,
            last_edited_by = $7, updated_at = now()
         WHERE id = $1
         RETURNING ",
		log_columns!()
	))
	.bind(id)
	.bind(fields.week_start)
	.bind(fields.week_end)
	.bind(fields.payday)
	.bind(fields.summary_total)
	.bind(fields.employee_cut)
	.bind(user_id)
	.bind(fields.pay_rate_percent)
	.bind(complete)
	.fetch_one(conn)
	.await
}

/// Replaces every load on the log (edits send the full set). A fixed number of statements
/// however many loads there are.
pub async fn replace_loads(
	conn: &mut PgConnection,
	log_id: Uuid,
	employee_id: Uuid,
	loads: &[LoadInput],
	user_id: Uuid,
) -> Result<(), sqlx::Error> {
	sqlx::query("DELETE FROM employee_loads WHERE paycheck_log_id = $1")
		.bind(log_id)
		.execute(&mut *conn)
		.await?;
	if loads.is_empty() {
		return Ok(());
	}

	let names: Vec<&str> = loads.iter().map(|load| load.store_name.as_str()).collect();
	let store_ids = resolve_store_ids(conn, &names, user_id).await?;
	let dates: Vec<NaiveDate> = loads.iter().map(|load| load.load_date).collect();
	let pays: Vec<Decimal> = loads.iter().map(|load| load.pay).collect();
	let extras: Vec<Decimal> = loads.iter().map(|load| load.extra_pay).collect();

	sqlx::query(
		"INSERT INTO employee_loads
			(employee_id, paycheck_log_id, store_id, load_date, pay, extra_pay,
			 created_by, last_edited_by)
		 SELECT $1, $2, store_id, load_date, pay, extra_pay, $3, $3
		 FROM unnest($4::uuid[], $5::date[], $6::numeric[], $7::numeric[])
			AS input(store_id, load_date, pay, extra_pay)",
	)
	.bind(employee_id)
	.bind(log_id)
	.bind(user_id)
	.bind(&store_ids)
	.bind(&dates)
	.bind(&pays)
	.bind(&extras)
	.execute(&mut *conn)
	.await?;

	Ok(())
}

/// Only flips the status: the totals already stored become the permanent snapshot.
pub async fn complete(
	conn: &mut PgConnection,
	id: Uuid,
	user_id: Uuid,
) -> Result<Option<PaycheckLog>, sqlx::Error> {
	sqlx::query_as(concat!(
		"UPDATE employee_paycheck_logs
         SET status = 'completed', last_edited_by = $2, updated_at = now()
         WHERE id = $1
         RETURNING ",
		log_columns!()
	))
	.bind(id)
	.bind(user_id)
	.fetch_optional(conn)
	.await
}

/// Loads go with the log (`ON DELETE CASCADE`). `false` when no log has this id.
pub async fn delete(conn: &mut PgConnection, id: Uuid) -> Result<bool, sqlx::Error> {
	let result = sqlx::query("DELETE FROM employee_paycheck_logs WHERE id = $1")
		.bind(id)
		.execute(conn)
		.await?;
	Ok(result.rows_affected() > 0)
}

pub async fn loads(pool: &PgPool, log_id: Uuid) -> Result<Vec<Load>, sqlx::Error> {
	sqlx::query_as(
		"SELECT employee_loads.id, stores.name AS store_name, load_date, pay, extra_pay
         FROM employee_loads
         JOIN stores ON stores.id = employee_loads.store_id
         WHERE paycheck_log_id = $1
         ORDER BY load_date",
	)
	.bind(log_id)
	.fetch_all(pool)
	.await
}

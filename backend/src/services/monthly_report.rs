//! The monthly report's numbers (EMAIL-SPEC.md §8) and the scheduler's run record. A paycheck
//! belongs to the month of its **payday** (when the money went out), unlike the dashboard,
//! which buckets by `week_start`.

use chrono::{Months, NaiveDate};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::models::monthly_report::{EmployeeMonth, MonthlyReportSource};

/// Completed paychecks with a payday in the month starting `month`, one row per employee
/// (inactive ones included: they were still paid), highest paid first.
async fn employee_months(
	pool: &PgPool,
	month: NaiveDate,
) -> Result<Vec<EmployeeMonth>, sqlx::Error> {
	sqlx::query_as(
		"SELECT e.first_name || ' ' || e.last_name AS name,
			count(*) AS paychecks,
			COALESCE(sum((SELECT count(*) FROM employee_loads el
				WHERE el.paycheck_log_id = pl.id)), 0)::bigint AS loads,
			COALESCE(sum(pl.summary_total), 0) AS gross,
			COALESCE(sum(pl.employee_cut), 0) AS paid
		 FROM employee_paycheck_logs pl
		 JOIN employees e ON e.id = pl.employee_id
		 WHERE pl.status = 'completed'
			AND pl.payday >= $1 AND pl.payday < ($1 + interval '1 month')::date
		 GROUP BY e.id, e.first_name, e.last_name
		 ORDER BY paid DESC, e.last_name, e.first_name",
	)
	.bind(month)
	.fetch_all(pool)
	.await
}

/// Everything the report shows for the month starting `month`, plus the month before it for
/// comparison.
pub async fn source(pool: &PgPool, month: NaiveDate) -> Result<MonthlyReportSource, sqlx::Error> {
	let previous_month = month - Months::new(1);
	let drafts: i64 = sqlx::query_scalar(
		"SELECT count(*) FROM employee_paycheck_logs
		 WHERE status = 'draft'
			AND payday >= $1 AND payday < ($1 + interval '1 month')::date",
	)
	.bind(month)
	.fetch_one(pool)
	.await?;

	Ok(MonthlyReportSource {
		month,
		employees: employee_months(pool, month).await?,
		previous: employee_months(pool, previous_month).await?,
		drafts,
	})
}

/// Records that the scheduler queued `month`'s report. `false` if it already had: the
/// caller must then queue nothing.
pub async fn claim_run(conn: &mut PgConnection, month: NaiveDate) -> Result<bool, sqlx::Error> {
	let inserted = sqlx::query(
		"INSERT INTO monthly_report_runs (month, outbox_ids) VALUES ($1, '{}')
		 ON CONFLICT (month) DO NOTHING",
	)
	.bind(month)
	.execute(&mut *conn)
	.await?;
	Ok(inserted.rows_affected() == 1)
}

pub async fn record_run_outbox_ids(
	conn: &mut PgConnection,
	month: NaiveDate,
	outbox_ids: &[Uuid],
) -> Result<(), sqlx::Error> {
	sqlx::query("UPDATE monthly_report_runs SET outbox_ids = $2 WHERE month = $1")
		.bind(month)
		.bind(outbox_ids)
		.execute(&mut *conn)
		.await?;
	Ok(())
}

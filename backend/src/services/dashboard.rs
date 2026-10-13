//! Year totals, per-employee and per-month breakdowns. A log belongs to the year its week
//! starts in. The year filter is a range (not `extract(year …)`) so it can use an index.

use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::dashboard::{EmployeeName, EmployeeTotal, MonthTotal};

pub struct YearTotals {
    pub ytd_total: Decimal,
    pub ytd_gross: Decimal,
    pub paycheck_count: i64,
    pub total_loads: i64,
}

pub async fn year_totals(pool: &PgPool, year: i32) -> Result<YearTotals, sqlx::Error> {
    let (ytd_total, ytd_gross, paycheck_count): (Decimal, Decimal, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(employee_cut), 0), COALESCE(SUM(summary_total), 0), COUNT(*)
         FROM employee_paycheck_logs
         WHERE status = 'completed'
         AND week_start >= make_date($1, 1, 1) AND week_start < make_date($1 + 1, 1, 1)",
    )
    .bind(year)
    .fetch_one(pool)
    .await?;

    let total_loads: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM employee_loads el
         JOIN employee_paycheck_logs pl ON pl.id = el.paycheck_log_id
         WHERE pl.status = 'completed'
         AND pl.week_start >= make_date($1, 1, 1) AND pl.week_start < make_date($1 + 1, 1, 1)",
    )
    .bind(year)
    .fetch_one(pool)
    .await?;

    Ok(YearTotals {
        ytd_total,
        ytd_gross,
        paycheck_count,
        total_loads,
    })
}

/// Highest earner first; ties broken by name so the order is stable.
pub async fn by_employee(pool: &PgPool, year: i32) -> Result<Vec<EmployeeTotal>, sqlx::Error> {
    sqlx::query_as(
        "SELECT e.id AS employee_id, e.first_name, e.last_name,
            COALESCE(SUM(pl.employee_cut), 0) AS total
         FROM employees e
         JOIN employee_paycheck_logs pl ON pl.employee_id = e.id
         WHERE pl.status = 'completed'
         AND pl.week_start >= make_date($1, 1, 1) AND pl.week_start < make_date($1 + 1, 1, 1)
         GROUP BY e.id, e.first_name, e.last_name
         ORDER BY total DESC, e.last_name, e.first_name",
    )
    .bind(year)
    .fetch_all(pool)
    .await
}

pub async fn by_month(pool: &PgPool, year: i32) -> Result<Vec<MonthTotal>, sqlx::Error> {
    sqlx::query_as(
        "SELECT to_char(date_trunc('month', pl.week_start), 'YYYY-MM') AS period,
            e.id AS employee_id, e.first_name, e.last_name,
            COALESCE(SUM(pl.employee_cut), 0) AS total
         FROM employee_paycheck_logs pl
         JOIN employees e ON e.id = pl.employee_id
         WHERE pl.status = 'completed'
         AND pl.week_start >= make_date($1, 1, 1) AND pl.week_start < make_date($1 + 1, 1, 1)
         GROUP BY period, e.id, e.first_name, e.last_name
         ORDER BY period, e.last_name, e.first_name",
    )
    .bind(year)
    .fetch_all(pool)
    .await
}

pub async fn employee_name(
    pool: &PgPool,
    employee_id: Uuid,
) -> Result<Option<EmployeeName>, sqlx::Error> {
    sqlx::query_as("SELECT id, first_name, last_name FROM employees WHERE id = $1")
        .bind(employee_id)
        .fetch_optional(pool)
        .await
}

pub async fn employee_ytd_total(
    pool: &PgPool,
    employee_id: Uuid,
    year: i32,
) -> Result<Decimal, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT COALESCE(SUM(employee_cut), 0)
         FROM employee_paycheck_logs
         WHERE employee_id = $1 AND status = 'completed'
         AND week_start >= make_date($2, 1, 1) AND week_start < make_date($2 + 1, 1, 1)",
    )
    .bind(employee_id)
    .bind(year)
    .fetch_one(pool)
    .await
}

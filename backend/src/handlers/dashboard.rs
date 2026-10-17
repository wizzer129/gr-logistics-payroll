//! /api/dashboard — admin only.

use axum::{Json, extract::State};
use chrono::{Datelike, Months, NaiveDate, Utc};

use crate::{
	auth::{AuthUser, Role},
	error::{AppError, AppResult, ErrorCode},
	extract::{ApiQuery, IdPath},
	models::dashboard::{DashboardSummary, EmployeeSummary, YearQuery},
	services::dashboard,
	state::AppState,
};

fn resolve_year(query: &YearQuery) -> AppResult<i32> {
	match query.year {
		None => Ok(Utc::now().year()),
		Some(year) if (2000..=2100).contains(&year) => Ok(year),
		Some(_) => Err(AppError::BadRequest(
			ErrorCode::InvalidYear,
			"Invalid year".into(),
		)),
	}
}

pub async fn summary(
	user: AuthUser,
	State(state): State<AppState>,
	ApiQuery(query): ApiQuery<YearQuery>,
) -> AppResult<Json<DashboardSummary>> {
	user.require_role(&[Role::Admin])?;
	let year = resolve_year(&query)?;

	// Independent reads: run them at once, each on its own pool connection.
	let (totals, by_employee, by_month) = tokio::try_join!(
		dashboard::year_totals(&state.pool, year),
		dashboard::by_employee(&state.pool, year),
		dashboard::by_month(&state.pool, year),
	)?;
	Ok(Json(DashboardSummary {
		year,
		ytd_total: totals.ytd_total,
		ytd_gross: totals.ytd_gross,
		paycheck_count: totals.paycheck_count,
		total_loads: totals.total_loads,
		by_employee,
		by_month,
	}))
}

pub async fn employee_summary(
	user: AuthUser,
	State(state): State<AppState>,
	IdPath(employee_id): IdPath,
	ApiQuery(query): ApiQuery<YearQuery>,
) -> AppResult<Json<EmployeeSummary>> {
	user.require_role(&[Role::Admin])?;
	let year = resolve_year(&query)?;

	let year_start = NaiveDate::from_ymd_opt(year, 1, 1).expect("year is validated");
	let month_start = Utc::now()
		.date_naive()
		.with_day(1)
		.expect("day 1 always exists");

	// Independent reads: run them at once. The totals of an unknown employee are just 0,
	// so the 404 is decided after.
	let (employee, ytd_total, month_total) = tokio::try_join!(
		dashboard::employee_name(&state.pool, employee_id),
		dashboard::employee_total(
			&state.pool,
			employee_id,
			year_start,
			year_start + Months::new(12),
		),
		dashboard::employee_total(
			&state.pool,
			employee_id,
			month_start,
			month_start + Months::new(1),
		),
	)?;
	let employee = employee.ok_or_else(|| {
		AppError::NotFound(ErrorCode::EmployeeNotFound, "Employee not found".into())
	})?;

	Ok(Json(EmployeeSummary {
		year,
		employee,
		ytd_total,
		month_total,
	}))
}

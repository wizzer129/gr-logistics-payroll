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

	let totals = dashboard::year_totals(&state.pool, year).await?;
	Ok(Json(DashboardSummary {
		year,
		ytd_total: totals.ytd_total,
		ytd_gross: totals.ytd_gross,
		paycheck_count: totals.paycheck_count,
		total_loads: totals.total_loads,
		by_employee: dashboard::by_employee(&state.pool, year).await?,
		by_month: dashboard::by_month(&state.pool, year).await?,
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

	let employee = dashboard::employee_name(&state.pool, employee_id)
		.await?
		.ok_or_else(|| {
			AppError::NotFound(ErrorCode::EmployeeNotFound, "Employee not found".into())
		})?;

	let year_start = NaiveDate::from_ymd_opt(year, 1, 1).expect("year is validated");
	let month_start = Utc::now()
		.date_naive()
		.with_day(1)
		.expect("day 1 always exists");

	Ok(Json(EmployeeSummary {
		year,
		employee,
		ytd_total: dashboard::employee_total(
			&state.pool,
			employee_id,
			year_start,
			year_start + Months::new(12),
		)
		.await?,
		month_total: dashboard::employee_total(
			&state.pool,
			employee_id,
			month_start,
			month_start + Months::new(1),
		)
		.await?,
	}))
}

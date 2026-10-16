//! /api/paycheck-logs — admin only. Every write (draft or completed) records an audit row
//! in the same transaction as the change.

use axum::{Json, extract::State, http::StatusCode};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde_json::json;
use uuid::Uuid;

use crate::{
	auth::{AuthUser, Role},
	domain::payment_calc::{calculate_employee_cut, sum_loads},
	error::{AppError, AppResult},
	extract::{ApiQuery, IdPath, ValidatedJson},
	models::{
		load::{Load, LoadInput},
		paycheck_log::{
			CreatePaycheckLog, PaycheckLog, PaycheckLogFilter, PaycheckLogListItem,
			UpdatePaycheckLog,
		},
	},
	services::{
		audit::{AuditAction, AuditEntry, record_audit},
		paycheck_logs::{self, LogFields},
	},
	state::AppState,
};

const ENTITY: &str = "employee_paycheck_log";
const DUPLICATE_WEEK: &str = "A paycheck log already exists for this employee and week";
/// `summary_total` is `numeric(10, 2)`; a bigger sum would fail in Postgres as a 500.
const MAX_TOTAL: Decimal = dec!(99999999.99);

fn not_found() -> AppError {
	AppError::NotFound("Paycheck log not found".into())
}

const KIND_LOADS: &str = "loads";
const KIND_LUMP_SUM: &str = "lump_sum";

/// What a create/update body pays for: itemized loads, or one lump sum (exactly one).
enum Payment<'a> {
	Loads(&'a [LoadInput]),
	LumpSum(Decimal),
}

impl<'a> Payment<'a> {
	fn from_body(loads: &'a [LoadInput], lump_sum: Option<Decimal>) -> AppResult<Self> {
		match (lump_sum, loads.is_empty()) {
			(Some(amount), true) => Ok(Self::LumpSum(amount)),
			(None, false) => Ok(Self::Loads(loads)),
			(Some(_), false) => Err(AppError::BadRequest(
				"Send either loads or lumpSum, not both".into(),
			)),
			(None, true) => Err(AppError::BadRequest(
				"loads: must have at least one load (or send lumpSum)".into(),
			)),
		}
	}

	fn kind(&self) -> &'static str {
		match self {
			Self::Loads(_) => KIND_LOADS,
			Self::LumpSum(_) => KIND_LUMP_SUM,
		}
	}

	/// Loads to store: none for a lump sum.
	fn loads(&self) -> &'a [LoadInput] {
		match self {
			Self::Loads(loads) => loads,
			Self::LumpSum(_) => &[],
		}
	}
}

/// Week range sanity plus the snapshot totals. Loads are cut at `pay_rate_percent`; a lump
/// sum is paid in full (total = cut = the amount, recorded at a 100% rate).
fn log_fields(
	week_start: NaiveDate,
	week_end: NaiveDate,
	payday: NaiveDate,
	payment: &Payment,
	pay_rate_percent: Decimal,
) -> AppResult<LogFields> {
	if week_end < week_start {
		return Err(AppError::BadRequest(
			"weekEnd must not be before weekStart".into(),
		));
	}

	let (summary_total, employee_cut, pay_rate_percent) = match payment {
		Payment::Loads(loads) => {
			let amounts: Vec<_> = loads.iter().map(LoadInput::amounts).collect();
			let summary_total = sum_loads(&amounts);
			if summary_total > MAX_TOTAL {
				return Err(AppError::BadRequest("Total pay is too large".into()));
			}
			let cut = calculate_employee_cut(summary_total, pay_rate_percent).rounded_pay;
			(summary_total, cut, pay_rate_percent)
		}
		Payment::LumpSum(amount) => (*amount, *amount, Decimal::ONE_HUNDRED),
	};

	Ok(LogFields {
		kind: payment.kind(),
		week_start,
		week_end,
		payday,
		summary_total,
		employee_cut,
		pay_rate_percent,
	})
}

fn audit(
	entity_id: Uuid,
	action: AuditAction,
	user: &AuthUser,
	diff: serde_json::Value,
) -> AuditEntry {
	AuditEntry {
		entity: ENTITY,
		entity_id,
		action,
		changed_by: user.id,
		diff,
	}
}

pub async fn list(
	user: AuthUser,
	State(state): State<AppState>,
	ApiQuery(filter): ApiQuery<PaycheckLogFilter>,
) -> AppResult<Json<Vec<PaycheckLogListItem>>> {
	user.require_role(&[Role::Admin])?;
	Ok(Json(paycheck_logs::list(&state.pool, &filter).await?))
}

pub async fn create(
	user: AuthUser,
	State(state): State<AppState>,
	ValidatedJson(body): ValidatedJson<CreatePaycheckLog>,
) -> AppResult<(StatusCode, Json<PaycheckLog>)> {
	user.require_role(&[Role::Admin])?;
	let payment = Payment::from_body(&body.loads, body.lump_sum)?;

	let mut tx = state.pool.begin().await?;

	let pay_rate = paycheck_logs::employee_pay_rate(&mut tx, body.employee_id)
		.await?
		.ok_or_else(|| AppError::NotFound("Employee not found".into()))?;
	let fields = log_fields(
		body.week_start,
		body.week_end,
		body.payday,
		&payment,
		pay_rate,
	)?;

	let status = if body.complete { "completed" } else { "draft" };
	let log = paycheck_logs::insert(&mut tx, body.employee_id, &fields, status, user.id)
		.await
		.map_err(|e| AppError::conflict_if_unique(e, DUPLICATE_WEEK))?;
	paycheck_logs::replace_loads(&mut tx, log.id, body.employee_id, payment.loads(), user.id)
		.await?;
	// A draft's create diff stays `{}`; created-as-completed records the status, like
	// `complete` does.
	let diff = if body.complete {
		json!({ "status": "completed" })
	} else {
		json!({})
	};
	record_audit(&mut tx, audit(log.id, AuditAction::Create, &user, diff)).await?;

	tx.commit().await?;
	Ok((StatusCode::CREATED, Json(log)))
}

/// Recomputes the totals from the new loads (or lump sum) at the rate stored on the log, not the
/// employee's current rate — editing never re-rates history. The edit dialog previews with
/// the same `payRatePercent` from the list response.
pub async fn update(
	user: AuthUser,
	State(state): State<AppState>,
	IdPath(id): IdPath,
	ValidatedJson(body): ValidatedJson<UpdatePaycheckLog>,
) -> AppResult<Json<PaycheckLog>> {
	user.require_role(&[Role::Admin])?;
	let payment = Payment::from_body(&body.loads, body.lump_sum)?;

	let mut tx = state.pool.begin().await?;

	let (employee_id, pay_rate, kind) = paycheck_logs::lock_for_update(&mut tx, id)
		.await?
		.ok_or_else(not_found)?;
	if kind != payment.kind() {
		return Err(AppError::BadRequest(format!(
			"This paycheck log is {kind}; it can't be edited into {}",
			payment.kind()
		)));
	}
	let fields = log_fields(
		body.week_start,
		body.week_end,
		body.payday,
		&payment,
		pay_rate,
	)?;

	let log = paycheck_logs::update(&mut tx, id, &fields, body.complete, user.id)
		.await
		.map_err(|e| AppError::conflict_if_unique(e, DUPLICATE_WEEK))?;
	paycheck_logs::replace_loads(&mut tx, id, employee_id, payment.loads(), user.id).await?;

	let diff = serde_json::to_value(&body).map_err(anyhow::Error::from)?;
	record_audit(&mut tx, audit(id, AuditAction::Update, &user, diff)).await?;

	tx.commit().await?;
	Ok(Json(log))
}

pub async fn delete(
	user: AuthUser,
	State(state): State<AppState>,
	IdPath(id): IdPath,
) -> AppResult<StatusCode> {
	user.require_role(&[Role::Admin])?;

	let mut tx = state.pool.begin().await?;
	if !paycheck_logs::delete(&mut tx, id).await? {
		return Err(not_found());
	}
	record_audit(&mut tx, audit(id, AuditAction::Delete, &user, json!({}))).await?;
	tx.commit().await?;

	Ok(StatusCode::NO_CONTENT)
}

/// Freezes `summary_total`/`employee_cut`: nothing recomputes them on its own afterwards,
/// so a later pay-rate change never alters a completed log.
pub async fn complete(
	user: AuthUser,
	State(state): State<AppState>,
	IdPath(id): IdPath,
) -> AppResult<Json<PaycheckLog>> {
	user.require_role(&[Role::Admin])?;

	let mut tx = state.pool.begin().await?;
	let log = paycheck_logs::complete(&mut tx, id, user.id)
		.await?
		.ok_or_else(not_found)?;
	record_audit(
		&mut tx,
		audit(
			id,
			AuditAction::Update,
			&user,
			json!({ "status": "completed" }),
		),
	)
	.await?;
	tx.commit().await?;

	Ok(Json(log))
}

/// Empty list for an unknown log id (the edit dialog treats it the same as no loads).
pub async fn loads(
	user: AuthUser,
	State(state): State<AppState>,
	IdPath(id): IdPath,
) -> AppResult<Json<Vec<Load>>> {
	user.require_role(&[Role::Admin])?;
	Ok(Json(paycheck_logs::loads(&state.pool, id).await?))
}

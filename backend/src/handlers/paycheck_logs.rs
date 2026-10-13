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

/// Week range sanity plus the snapshot totals for a set of loads at a given pay rate.
fn log_fields(
    week_start: NaiveDate,
    week_end: NaiveDate,
    payday: NaiveDate,
    loads: &[LoadInput],
    pay_rate_percent: Decimal,
) -> AppResult<LogFields> {
    if week_end < week_start {
        return Err(AppError::BadRequest(
            "weekEnd must not be before weekStart".into(),
        ));
    }

    let amounts: Vec<_> = loads.iter().map(LoadInput::amounts).collect();
    let summary_total = sum_loads(&amounts);
    if summary_total > MAX_TOTAL {
        return Err(AppError::BadRequest("Total pay is too large".into()));
    }

    Ok(LogFields {
        week_start,
        week_end,
        payday,
        summary_total,
        employee_cut: calculate_employee_cut(summary_total, pay_rate_percent).rounded_pay,
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

    let mut tx = state.pool.begin().await?;

    let pay_rate = paycheck_logs::employee_pay_rate(&mut tx, body.employee_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Employee not found".into()))?;
    let fields = log_fields(
        body.week_start,
        body.week_end,
        body.payday,
        &body.loads,
        pay_rate,
    )?;

    let log = paycheck_logs::insert(&mut tx, body.employee_id, &fields, user.id)
        .await
        .map_err(|e| AppError::conflict_if_unique(e, DUPLICATE_WEEK))?;
    paycheck_logs::replace_loads(&mut tx, log.id, body.employee_id, &body.loads, user.id).await?;
    record_audit(
        &mut tx,
        audit(log.id, AuditAction::Create, &user, json!({})),
    )
    .await?;

    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(log)))
}

/// Recomputes the totals from the new loads at the rate stored on the log, not the
/// employee's current rate — editing never re-rates history. The edit dialog previews with
/// the same `payRatePercent` from the list response.
pub async fn update(
    user: AuthUser,
    State(state): State<AppState>,
    IdPath(id): IdPath,
    ValidatedJson(body): ValidatedJson<UpdatePaycheckLog>,
) -> AppResult<Json<PaycheckLog>> {
    user.require_role(&[Role::Admin])?;

    let mut tx = state.pool.begin().await?;

    let (employee_id, pay_rate) = paycheck_logs::lock_for_update(&mut tx, id)
        .await?
        .ok_or_else(not_found)?;
    let fields = log_fields(
        body.week_start,
        body.week_end,
        body.payday,
        &body.loads,
        pay_rate,
    )?;

    let log = paycheck_logs::update(&mut tx, id, &fields, user.id)
        .await
        .map_err(|e| AppError::conflict_if_unique(e, DUPLICATE_WEEK))?;
    paycheck_logs::replace_loads(&mut tx, id, employee_id, &body.loads, user.id).await?;

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

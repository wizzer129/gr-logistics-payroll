//! /api/employees — admin only. Every write records an audit row in the same transaction.

use axum::{Json, extract::State, http::StatusCode};
use serde_json::json;

use crate::{
	auth::{AuthUser, Role},
	error::{AppError, AppResult, ErrorCode},
	extract::{IdPath, ValidatedJson},
	models::employee::{CreateEmployee, Employee, UpdateEmployee},
	services::{
		audit::{AuditAction, AuditEntry, record_audit},
		employees,
	},
	state::AppState,
};

const ENTITY: &str = "employee";

fn not_found() -> AppError {
	AppError::NotFound(ErrorCode::EmployeeNotFound, "Employee not found".into())
}

pub async fn list(user: AuthUser, State(state): State<AppState>) -> AppResult<Json<Vec<Employee>>> {
	user.require_role(&[Role::Admin])?;
	Ok(Json(employees::list_active(&state.pool).await?))
}

pub async fn create(
	user: AuthUser,
	State(state): State<AppState>,
	ValidatedJson(body): ValidatedJson<CreateEmployee>,
) -> AppResult<(StatusCode, Json<Employee>)> {
	user.require_role(&[Role::Admin])?;

	let mut tx = state.pool.begin().await?;
	let employee = employees::create(&mut tx, &body, user.id).await?;
	record_audit(
		&mut tx,
		AuditEntry {
			entity: ENTITY,
			entity_id: employee.id,
			action: AuditAction::Create,
			changed_by: user.id,
			diff: serde_json::to_value(&body).map_err(anyhow::Error::from)?,
		},
	)
	.await?;
	tx.commit().await?;

	Ok((StatusCode::CREATED, Json(employee)))
}

pub async fn update(
	user: AuthUser,
	State(state): State<AppState>,
	IdPath(id): IdPath,
	ValidatedJson(body): ValidatedJson<UpdateEmployee>,
) -> AppResult<Json<Employee>> {
	user.require_role(&[Role::Admin])?;

	let mut tx = state.pool.begin().await?;
	let employee = employees::update(&mut tx, id, &body, user.id)
		.await?
		.ok_or_else(not_found)?;
	record_audit(
		&mut tx,
		AuditEntry {
			entity: ENTITY,
			entity_id: id,
			action: AuditAction::Update,
			changed_by: user.id,
			diff: serde_json::to_value(&body).map_err(anyhow::Error::from)?,
		},
	)
	.await?;
	tx.commit().await?;

	Ok(Json(employee))
}

/// Soft delete: the employee disappears from the list but keeps every load and log.
pub async fn delete(
	user: AuthUser,
	State(state): State<AppState>,
	IdPath(id): IdPath,
) -> AppResult<StatusCode> {
	user.require_role(&[Role::Admin])?;

	let mut tx = state.pool.begin().await?;
	if !employees::soft_delete(&mut tx, id, user.id).await? {
		return Err(not_found());
	}
	record_audit(
		&mut tx,
		AuditEntry {
			entity: ENTITY,
			entity_id: id,
			action: AuditAction::Delete,
			changed_by: user.id,
			diff: json!({}),
		},
	)
	.await?;
	tx.commit().await?;

	Ok(StatusCode::NO_CONTENT)
}

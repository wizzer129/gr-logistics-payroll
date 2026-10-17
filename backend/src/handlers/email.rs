//! /api/email — the email controller (EMAIL-SPEC.md §9). Admin only.

use axum::{Json, extract::State, http::StatusCode};

use crate::{
	auth::{AuthUser, Role},
	email::{self, Message},
	error::{AppError, AppResult, ErrorCode},
	extract::{IdPath, ValidatedJson},
	models::email::{
		MonthlyReportRecipients, OutboxEntry, Queued, RecipientOptions, SendMonthlyReport, SendTo,
	},
	models::paycheck_log::PaycheckStatus,
	services::{
		email_outbox::{self, Retry},
		email_recipients, paystubs,
	},
	state::AppState,
};

/// GET /api/email/recipient-options — what `RecipientSelect` offers.
pub async fn recipient_options(
	user: AuthUser,
	State(state): State<AppState>,
) -> AppResult<Json<RecipientOptions>> {
	user.require_role(&[Role::Admin])?;
	Ok(Json(email_recipients::options(&state.pool).await?))
}

/// GET /api/email/monthly-report/recipients
pub async fn monthly_report_recipients(
	user: AuthUser,
	State(state): State<AppState>,
) -> AppResult<Json<MonthlyReportRecipients>> {
	user.require_role(&[Role::Admin])?;
	let user_ids = email_recipients::monthly_report_user_ids(&state.pool).await?;
	Ok(Json(MonthlyReportRecipients { user_ids }))
}

/// PUT /api/email/monthly-report/recipients — replaces the whole set. Users only: the report
/// shows everyone's pay, so employees can't receive it.
pub async fn set_monthly_report_recipients(
	user: AuthUser,
	State(state): State<AppState>,
	ValidatedJson(body): ValidatedJson<MonthlyReportRecipients>,
) -> AppResult<Json<MonthlyReportRecipients>> {
	user.require_role(&[Role::Admin])?;

	let mut tx = state.pool.begin().await?;
	if !email_recipients::set_monthly_report_users(&mut tx, &body.user_ids).await? {
		return Err(AppError::BadRequest(
			ErrorCode::RecipientNotFound,
			"Unknown user id".into(),
		));
	}
	tx.commit().await?;

	let user_ids = email_recipients::monthly_report_user_ids(&state.pool).await?;
	Ok(Json(MonthlyReportRecipients { user_ids }))
}

/// POST /api/paycheck-logs/{id}/email — queues the pay stub (PDF attached) for each picked
/// recipient, in their own language. Completed logs only: a draft isn't a pay stub yet.
pub async fn send_paystub(
	user: AuthUser,
	State(state): State<AppState>,
	IdPath(id): IdPath,
	ValidatedJson(body): ValidatedJson<SendTo>,
) -> AppResult<(StatusCode, Json<Queued>)> {
	user.require_role(&[Role::Admin])?;

	match paystubs::status(&state.pool, id).await? {
		None => {
			return Err(AppError::NotFound(
				ErrorCode::PaycheckLogNotFound,
				"Paycheck log not found".into(),
			));
		}
		Some(PaycheckStatus::Completed) => {}
		Some(_) => {
			return Err(AppError::Conflict(
				ErrorCode::PaycheckNotCompleted,
				"Only a completed paycheck log can be emailed".into(),
			));
		}
	}

	let mut tx = state.pool.begin().await?;
	let recipients = email_recipients::resolve(&mut tx, &body.user_ids, &body.employee_ids)
		.await?
		.ok_or_else(|| {
			AppError::BadRequest(ErrorCode::RecipientNotFound, "Unknown recipient".into())
		})?;
	if recipients.is_empty() {
		return Err(AppError::BadRequest(
			ErrorCode::NoRecipients,
			"Pick at least one recipient".into(),
		));
	}

	let message = Message::Paystub {
		paycheck_log_id: id,
	};
	let outbox_ids = email::enqueue_for_each(&mut tx, &message, &recipients, Some(user.id)).await?;
	tx.commit().await?;

	Ok((StatusCode::ACCEPTED, Json(Queued { outbox_ids })))
}

/// POST /api/email/monthly-report — queues a month's report now, whatever the scheduler has
/// done: for a resend, an earlier month, or a month outside its catch-up window.
pub async fn send_monthly_report(
	user: AuthUser,
	State(state): State<AppState>,
	ValidatedJson(body): ValidatedJson<SendMonthlyReport>,
) -> AppResult<(StatusCode, Json<Queued>)> {
	user.require_role(&[Role::Admin])?;
	let month = body
		.first_day()
		.ok_or_else(|| anyhow::anyhow!("month passed validation but didn't parse"))?;

	let mut tx = state.pool.begin().await?;
	let recipients = match &body.user_ids {
		Some(user_ids) => email_recipients::resolve(&mut tx, user_ids, &[])
			.await?
			.ok_or_else(|| {
				AppError::BadRequest(ErrorCode::RecipientNotFound, "Unknown user id".into())
			})?,
		None => email_recipients::monthly_report_recipients(&mut tx).await?,
	};
	if recipients.is_empty() {
		return Err(AppError::BadRequest(
			ErrorCode::NoRecipients,
			"Nobody to send the report to".into(),
		));
	}

	let message = Message::MonthlyReport { month };
	let outbox_ids = email::enqueue_for_each(&mut tx, &message, &recipients, Some(user.id)).await?;
	tx.commit().await?;

	Ok((StatusCode::ACCEPTED, Json(Queued { outbox_ids })))
}

/// How many rows the "Sent emails" panel shows.
const RECENT_EMAILS: i64 = 100;

/// GET /api/email/outbox — the most recent emails and how each one went.
pub async fn outbox(
	user: AuthUser,
	State(state): State<AppState>,
) -> AppResult<Json<Vec<OutboxEntry>>> {
	user.require_role(&[Role::Admin])?;
	Ok(Json(
		email_outbox::recent(&state.pool, RECENT_EMAILS).await?,
	))
}

/// POST /api/email/outbox/{id}/retry — sends a failed email again.
pub async fn retry_email(
	user: AuthUser,
	State(state): State<AppState>,
	IdPath(id): IdPath,
) -> AppResult<StatusCode> {
	user.require_role(&[Role::Admin])?;

	let mut tx = state.pool.begin().await?;
	match email_outbox::retry(&mut tx, id).await? {
		Retry::Queued => {}
		Retry::NotFound => {
			return Err(AppError::NotFound(
				ErrorCode::OutboxEmailNotFound,
				"Email not found".into(),
			));
		}
		Retry::NotRetryable => {
			return Err(AppError::Conflict(
				ErrorCode::EmailNotRetryable,
				"Only a failed email that can still be rebuilt can be retried".into(),
			));
		}
	}
	tx.commit().await?;
	Ok(StatusCode::ACCEPTED)
}

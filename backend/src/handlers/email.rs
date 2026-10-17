//! /api/email — the email controller (EMAIL-SPEC.md §9). Admin only.

use axum::{Json, extract::State, http::StatusCode};

use crate::{
	auth::{AuthUser, Role},
	email::{self, Message},
	error::{AppError, AppResult, ErrorCode},
	extract::{IdPath, ValidatedJson},
	models::email::{MonthlyReportRecipients, Queued, RecipientOptions, SendTo},
	services::{email_recipients, paystubs},
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

	match paystubs::status(&state.pool, id).await?.as_deref() {
		None => {
			return Err(AppError::NotFound(
				ErrorCode::PaycheckLogNotFound,
				"Paycheck log not found".into(),
			));
		}
		Some("completed") => {}
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
	let mut outbox_ids = Vec::with_capacity(recipients.len());
	for recipient in &recipients {
		outbox_ids.push(
			email::enqueue(
				&mut tx,
				&message,
				&recipient.email,
				recipient.locale,
				Some(user.id),
			)
			.await?,
		);
	}
	tx.commit().await?;

	Ok((StatusCode::ACCEPTED, Json(Queued { outbox_ids })))
}

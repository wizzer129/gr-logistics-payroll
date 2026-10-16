//! /api/email — the email controller (EMAIL-SPEC.md §9). Admin only.

use axum::{Json, extract::State};

use crate::{
	auth::{AuthUser, Role},
	error::{AppError, AppResult, ErrorCode},
	extract::ValidatedJson,
	models::email::{MonthlyReportRecipients, RecipientOptions},
	services::email_recipients,
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

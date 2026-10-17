//! DTOs for the email controller (`handlers/email.rs`, EMAIL-SPEC.md §6 and §9).

use garde::Validate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct UserOption {
	pub id: Uuid,
	pub email: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeOption {
	pub id: Uuid,
	pub first_name: String,
	pub last_name: String,
	pub email: String,
}

/// Everyone an email can go to: every user, and active employees that have an email.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipientOptions {
	pub users: Vec<UserOption>,
	pub employees: Vec<EmployeeOption>,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct MonthlyReportRecipients {
	#[garde(skip)]
	pub user_ids: Vec<Uuid>,
}

/// Who to email: picked users and/or employees (never typed addresses).
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SendTo {
	#[serde(default)]
	#[garde(skip)]
	pub user_ids: Vec<Uuid>,
	#[serde(default)]
	#[garde(skip)]
	pub employee_ids: Vec<Uuid>,
}

/// One outbox row per recipient.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Queued {
	pub outbox_ids: Vec<Uuid>,
}

//! DTOs for the email controller (`handlers/email.rs`, EMAIL-SPEC.md §6 and §9).

use chrono::{DateTime, NaiveDate, Utc};
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

/// Who to email: picked users and/or employees, plus addresses typed in by hand (written in
/// the sender's own language, since there's no saved one).
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SendTo {
	#[serde(default)]
	#[garde(skip)]
	pub user_ids: Vec<Uuid>,
	#[serde(default)]
	#[garde(skip)]
	pub employee_ids: Vec<Uuid>,
	#[serde(default)]
	#[garde(length(max = MAX_TYPED_EMAILS), inner(email))]
	pub emails: Vec<String>,
}

pub const MAX_TYPED_EMAILS: usize = 20;
/// Pay stubs in one bulk email: each is a PDF attachment, and providers cap the total size.
pub const MAX_PAYSTUBS_PER_EMAIL: usize = 50;

/// `POST /api/email/paystubs`: several completed logs' pay stubs in one email per recipient.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SendPaystubs {
	#[garde(length(min = 1, max = MAX_PAYSTUBS_PER_EMAIL))]
	pub paycheck_log_ids: Vec<Uuid>,
	#[serde(flatten)]
	#[garde(dive)]
	pub to: SendTo,
}

/// One outbox row per recipient.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Queued {
	pub outbox_ids: Vec<Uuid>,
}

/// `POST /api/email/monthly-report`: send a month's report now. Without `userIds` it goes to
/// the opted-in users.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SendMonthlyReport {
	/// `YYYY-MM`.
	#[garde(custom(year_month))]
	pub month: String,
	#[serde(default)]
	#[garde(skip)]
	pub user_ids: Option<Vec<Uuid>>,
}

impl SendMonthlyReport {
	/// The month's 1st. Only call after validation.
	pub fn first_day(&self) -> Option<NaiveDate> {
		parse_year_month(&self.month)
	}
}

fn parse_year_month(value: &str) -> Option<NaiveDate> {
	if value.len() != 7 {
		return None;
	}
	NaiveDate::parse_from_str(&format!("{value}-01"), "%Y-%m-%d").ok()
}

fn year_month(value: &str, _ctx: &()) -> garde::Result {
	match parse_year_month(value) {
		Some(_) => Ok(()),
		None => Err(garde::Error::new("must be a month as YYYY-MM")),
	}
}

/// One row of the "Sent emails" panel.
#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct OutboxEntry {
	pub id: Uuid,
	pub kind: String,
	pub recipient: String,
	pub status: String,
	pub attempts: i32,
	pub subject: Option<String>,
	pub last_error: Option<String>,
	pub created_at: DateTime<Utc>,
	pub sent_at: Option<DateTime<Utc>>,
	pub next_attempt_at: DateTime<Utc>,
	/// A failed email that still has what it needs to be rebuilt (a sent or failed password
	/// reset has had its link scrubbed, so it can't be).
	pub retryable: bool,
}

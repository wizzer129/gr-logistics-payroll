//! Who email can go to (EMAIL-SPEC.md §6): existing users and active employees with an email,
//! each with the language their email is written in.

use std::collections::HashSet;

use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::{email::Locale, models::email::RecipientOptions};

pub async fn options(pool: &PgPool) -> Result<RecipientOptions, sqlx::Error> {
	let users = sqlx::query_as("SELECT id, email FROM users ORDER BY email")
		.fetch_all(pool)
		.await?;
	let employees = sqlx::query_as(
		"SELECT id, first_name, last_name, email FROM employees
		 WHERE active AND email IS NOT NULL
		 ORDER BY last_name, first_name",
	)
	.fetch_all(pool)
	.await?;
	Ok(RecipientOptions { users, employees })
}

pub async fn set_user_locale(
	pool: &PgPool,
	user_id: Uuid,
	locale: Locale,
) -> Result<(), sqlx::Error> {
	sqlx::query("UPDATE users SET locale = $2 WHERE id = $1")
		.bind(user_id)
		.bind(locale.as_str())
		.execute(pool)
		.await?;
	Ok(())
}

pub async fn monthly_report_user_ids(pool: &PgPool) -> Result<Vec<Uuid>, sqlx::Error> {
	sqlx::query_scalar("SELECT id FROM users WHERE receives_monthly_report ORDER BY email")
		.fetch_all(pool)
		.await
}

/// The users opted in to the monthly report, as addresses.
pub async fn monthly_report_recipients(
	conn: &mut PgConnection,
) -> Result<Vec<Recipient>, sqlx::Error> {
	let rows: Vec<(String, String)> = sqlx::query_as(
		"SELECT email, locale FROM users WHERE receives_monthly_report ORDER BY email",
	)
	.fetch_all(&mut *conn)
	.await?;
	Ok(rows
		.into_iter()
		.map(|(email, locale)| Recipient {
			email,
			locale: Locale::parse(&locale).unwrap_or(Locale::En),
		})
		.collect())
}

/// Makes exactly `user_ids` the monthly report's recipients. `false` (and no change) if any
/// id isn't a user.
pub async fn set_monthly_report_users(
	conn: &mut PgConnection,
	user_ids: &[Uuid],
) -> Result<bool, sqlx::Error> {
	let known: i64 = sqlx::query_scalar("SELECT count(*) FROM users WHERE id = ANY($1)")
		.bind(user_ids)
		.fetch_one(&mut *conn)
		.await?;
	if known != unique(user_ids) {
		return Ok(false);
	}
	sqlx::query("UPDATE users SET receives_monthly_report = (id = ANY($1))")
		.bind(user_ids)
		.execute(&mut *conn)
		.await?;
	Ok(true)
}

/// An address an email goes to, in its recipient's language.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recipient {
	pub email: String,
	pub locale: Locale,
}

/// Turns picked ids into addresses. `None` if any id isn't a user, or an active employee with
/// an email. One person picked twice (e.g. as a user and as an employee with the same
/// address) gets one email, in the first match's language.
pub async fn resolve(
	conn: &mut PgConnection,
	user_ids: &[Uuid],
	employee_ids: &[Uuid],
) -> Result<Option<Vec<Recipient>>, sqlx::Error> {
	let users: Vec<(String, String)> =
		sqlx::query_as("SELECT email, locale FROM users WHERE id = ANY($1) ORDER BY email")
			.bind(user_ids)
			.fetch_all(&mut *conn)
			.await?;
	let employees: Vec<(String, String)> = sqlx::query_as(
		"SELECT email, locale FROM employees
		 WHERE id = ANY($1) AND active AND email IS NOT NULL
		 ORDER BY email",
	)
	.bind(employee_ids)
	.fetch_all(&mut *conn)
	.await?;

	if users.len() as i64 != unique(user_ids) || employees.len() as i64 != unique(employee_ids) {
		return Ok(None);
	}

	let mut seen = HashSet::new();
	Ok(Some(
		users
			.into_iter()
			.chain(employees)
			.filter(|(email, _)| seen.insert(email.to_lowercase()))
			.map(|(email, locale)| Recipient {
				email,
				locale: Locale::parse(&locale).unwrap_or(Locale::En),
			})
			.collect(),
	))
}

fn unique(ids: &[Uuid]) -> i64 {
	ids.iter().collect::<HashSet<_>>().len() as i64
}

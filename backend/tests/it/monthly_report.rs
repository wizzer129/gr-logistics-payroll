//! The monthly report (EMAIL-SPEC.md §8): the numbers, the scheduler, "Send now", and the
//! "Sent emails" panel with its retry.

use std::sync::Arc;

use crate::common::{app_with_db, insert_user, login, send};
use axum::Router;
use chrono::{TimeZone, Utc};
use gr_logistics::email::{
	outbox::Outbox,
	render::Renderer,
	scheduler::{self, TIMEZONE},
	sender::{EmailSender, MemorySender, SendError},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

struct Ctx {
	app: Router,
	pool: PgPool,
	cookie: String,
	sender: Arc<MemorySender>,
	admin: Uuid,
}

impl Ctx {
	async fn new(pool: PgPool) -> Self {
		let app = app_with_db(pool.clone()).await;
		let admin = insert_user(&pool, "admin@example.com", "admin").await;
		let cookie = login(&app, "admin@example.com").await;
		Self {
			app,
			pool,
			cookie,
			sender: Arc::new(MemorySender::default()),
			admin,
		}
	}

	async fn call(&self, method: &str, uri: &str, body: Value) -> (u16, Value) {
		let body = body.to_string();
		let response = send(&self.app, method, uri, Some(&body), Some(&self.cookie)).await;
		(response.status.as_u16(), response.body)
	}

	async fn employee(&self, first: &str) -> Uuid {
		let (status, body) = self
			.call(
				"POST",
				"/api/employees",
				json!({ "firstName": first, "lastName": "Test", "payRatePercent": 42 }),
			)
			.await;
		assert_eq!(status, 201, "{body:?}");
		body["id"].as_str().unwrap().parse().unwrap()
	}

	/// A one-load week paying `pay` (42% → the cut), paid on `payday`.
	async fn paycheck(
		&self,
		employee: Uuid,
		week_start: &str,
		payday: &str,
		pay: u32,
		complete: bool,
	) {
		let start = chrono::NaiveDate::parse_from_str(week_start, "%Y-%m-%d").unwrap();
		let end = start + chrono::Days::new(6);
		let (status, body) = self
			.call(
				"POST",
				"/api/paycheck-logs",
				json!({
					"employeeId": employee, "weekStart": start, "weekEnd": end,
					"payday": payday, "complete": complete,
					"loads": [{ "storeName": "Store A", "loadDate": start, "pay": pay }],
				}),
			)
			.await;
		assert_eq!(status, 201, "{body:?}");
	}

	async fn opt_in(&self, user_ids: &[Uuid]) {
		let (status, _) = self
			.call(
				"PUT",
				"/api/email/monthly-report/recipients",
				json!({ "userIds": user_ids }),
			)
			.await;
		assert_eq!(status, 200);
	}

	async fn deliver(&self) {
		Outbox::new(
			self.pool.clone(),
			self.sender.clone() as Arc<dyn EmailSender>,
			Arc::new(Renderer::new()),
		)
		.process_due()
		.await
		.unwrap();
	}

	async fn outbox_count(&self) -> i64 {
		sqlx::query_scalar("SELECT count(*) FROM email_outbox")
			.fetch_one(&self.pool)
			.await
			.unwrap()
	}
}

/// Los Angeles wall-clock time, as UTC.
fn la(y: i32, m: u32, d: u32, h: u32) -> chrono::DateTime<Utc> {
	TIMEZONE
		.with_ymd_and_hms(y, m, d, h, 0, 0)
		.unwrap()
		.with_timezone(&Utc)
}

#[sqlx::test(migrations = false)]
async fn the_report_counts_paychecks_by_payday_and_notes_drafts(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let ana = ctx.employee("Ana").await;
	let luis = ctx.employee("Luis").await;
	// September by payday, though this week started in August.
	ctx.paycheck(ana, "2026-08-30", "2026-09-04", 1000, true)
		.await; // $420
	ctx.paycheck(ana, "2026-09-06", "2026-09-11", 1000, true)
		.await; // $420
	ctx.paycheck(luis, "2026-09-13", "2026-09-18", 500, true)
		.await; // $210
	ctx.paycheck(luis, "2026-09-20", "2026-09-25", 900, false)
		.await; // draft: not counted
	// August, for the comparison; October, which must not count.
	ctx.paycheck(ana, "2026-08-09", "2026-08-14", 1000, true)
		.await; // $420
	ctx.paycheck(ana, "2026-09-27", "2026-10-02", 2000, true)
		.await;

	let (status, body) = ctx
		.call(
			"POST",
			"/api/email/monthly-report",
			json!({ "month": "2026-09" }),
		)
		.await;
	assert_eq!(status, 400, "nobody opted in yet");
	assert_eq!(body["code"], "NO_RECIPIENTS");

	ctx.opt_in(&[ctx.admin]).await;
	let (status, body) = ctx
		.call(
			"POST",
			"/api/email/monthly-report",
			json!({ "month": "2026-09" }),
		)
		.await;
	assert_eq!(status, 202, "{body:?}");
	ctx.deliver().await;

	let sent = ctx.sender.sent();
	assert_eq!(sent.len(), 1);
	let email = &sent[0];
	assert_eq!(email.to, "admin@example.com");
	assert_eq!(email.subject, "Monthly report: September 2026");
	assert!(email.html.contains("$1,050.00"), "420 + 420 + 210");
	assert!(
		email.html.contains("+150% vs. August 2026"),
		"{}",
		email.text
	);
	assert!(email.html.contains("$2,500.00"), "gross");
	assert!(
		email
			.text
			.contains("1 paycheck with a payday in September 2026 is still a draft"),
		"{}",
		email.text
	);
	assert_eq!(email.attachments[0].filename, "monthly-report-2026-09.pdf");
	assert!(email.attachments[0].bytes.starts_with(b"%PDF"));
}

#[sqlx::test(migrations = false)]
async fn send_now_picks_users_in_their_language(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let printer = insert_user(&ctx.pool, "printer@example.com", "employee").await;
	sqlx::query("UPDATE users SET locale = 'es' WHERE id = $1")
		.bind(printer)
		.execute(&ctx.pool)
		.await
		.unwrap();

	let (status, body) = ctx
		.call(
			"POST",
			"/api/email/monthly-report",
			json!({ "month": "2026-09", "userIds": [printer] }),
		)
		.await;
	assert_eq!(status, 202, "{body:?}");
	ctx.deliver().await;
	let sent = ctx.sender.sent();
	assert_eq!(sent.len(), 1, "only the picked user, not the opted-in ones");
	assert_eq!(sent[0].subject, "Reporte mensual: septiembre de 2026");
	assert_eq!(
		sent[0].attachments[0].filename,
		"reporte-mensual-2026-09.pdf"
	);
	assert!(!sent[0].text.contains("borrador"), "no drafts, no note");

	for (body, code) in [
		(json!({ "month": "2026-13" }), "VALIDATION_FAILED"),
		(json!({ "month": "2026-9" }), "VALIDATION_FAILED"),
		(
			json!({ "month": "2026-09", "userIds": [Uuid::new_v4()] }),
			"RECIPIENT_NOT_FOUND",
		),
		(
			json!({ "month": "2026-09", "userIds": [] }),
			"NO_RECIPIENTS",
		),
	] {
		let (status, response) = ctx
			.call("POST", "/api/email/monthly-report", body.clone())
			.await;
		assert_eq!(status, 400, "{body}");
		assert_eq!(response["code"], code, "{body}");
	}

	let cookie = login(&ctx.app, "printer@example.com").await;
	let body = json!({ "month": "2026-09" }).to_string();
	for (method, uri, body) in [
		("POST", "/api/email/monthly-report", Some(body.as_str())),
		("GET", "/api/email/outbox", None),
	] {
		let response = send(&ctx.app, method, uri, body, Some(&cookie)).await;
		assert_eq!(response.status, 403, "{method} {uri}");
	}
}

#[sqlx::test(migrations = false)]
async fn the_scheduler_queues_last_month_once(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let first = la(2026, 10, 1, 8);

	assert_eq!(
		scheduler::tick(&ctx.pool, first).await.unwrap(),
		None,
		"nobody opted in: nothing queued, nothing recorded"
	);

	ctx.opt_in(&[ctx.admin]).await;
	assert_eq!(
		scheduler::tick(&ctx.pool, la(2026, 10, 1, 7))
			.await
			.unwrap(),
		None,
		"too early"
	);
	let ids = scheduler::tick(&ctx.pool, first)
		.await
		.unwrap()
		.expect("queued");
	assert_eq!(ids.len(), 1);
	assert_eq!(
		scheduler::tick(&ctx.pool, la(2026, 10, 1, 9))
			.await
			.unwrap(),
		None,
		"the next hour"
	);
	assert_eq!(
		scheduler::tick(&ctx.pool, la(2026, 10, 3, 9))
			.await
			.unwrap(),
		None,
		"after a restart"
	);
	assert_eq!(ctx.outbox_count().await, 1);

	let (month, recorded): (chrono::NaiveDate, Vec<Uuid>) =
		sqlx::query_as("SELECT month, outbox_ids FROM monthly_report_runs")
			.fetch_one(&ctx.pool)
			.await
			.unwrap();
	assert_eq!(month.to_string(), "2026-09-01");
	assert_eq!(recorded, ids);

	ctx.deliver().await;
	assert_eq!(
		ctx.sender.sent()[0].subject,
		"Monthly report: September 2026"
	);

	// "Send now" ignores the run record.
	let (status, _) = ctx
		.call(
			"POST",
			"/api/email/monthly-report",
			json!({ "month": "2026-09" }),
		)
		.await;
	assert_eq!(status, 202);
	assert_eq!(ctx.outbox_count().await, 2);
}

#[sqlx::test(migrations = false)]
async fn failed_emails_are_listed_and_can_be_retried(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	ctx.opt_in(&[ctx.admin]).await;
	ctx.call(
		"POST",
		"/api/email/monthly-report",
		json!({ "month": "2026-09" }),
	)
	.await;
	*ctx.sender.fail_with.lock().unwrap() = Some(SendError::Permanent("422 invalid to".into()));
	ctx.deliver().await;

	let (status, list) = ctx.call("GET", "/api/email/outbox", Value::Null).await;
	assert_eq!(status, 200);
	let row = &list[0];
	assert_eq!(row["kind"], "monthly_report");
	assert_eq!(row["recipient"], "admin@example.com");
	assert_eq!(row["status"], "failed");
	assert_eq!(row["lastError"], "422 invalid to");
	assert_eq!(row["retryable"], true);
	let id = row["id"].as_str().unwrap().to_owned();

	*ctx.sender.fail_with.lock().unwrap() = None;
	let retry = format!("/api/email/outbox/{id}/retry");
	let (status, _) = ctx.call("POST", &retry, Value::Null).await;
	assert_eq!(status, 202);
	ctx.deliver().await;
	assert_eq!(ctx.sender.sent().len(), 1);

	let (_, list) = ctx.call("GET", "/api/email/outbox", Value::Null).await;
	assert_eq!(list[0]["status"], "sent");
	assert_eq!(list[0]["attempts"], 1, "a retry starts its attempts over");
	assert_eq!(list[0]["retryable"], false);

	let (status, body) = ctx.call("POST", &retry, Value::Null).await;
	assert_eq!(status, 409, "already sent");
	assert_eq!(body["code"], "EMAIL_NOT_RETRYABLE");
	let (status, body) = ctx
		.call(
			"POST",
			&format!("/api/email/outbox/{}/retry", Uuid::new_v4()),
			Value::Null,
		)
		.await;
	assert_eq!(status, 404);
	assert_eq!(body["code"], "OUTBOX_EMAIL_NOT_FOUND");
}

#[sqlx::test(migrations = false)]
async fn a_failed_password_reset_cannot_be_retried(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	*ctx.sender.fail_with.lock().unwrap() = Some(SendError::Permanent("bounced".into()));
	let body = json!({ "email": "admin@example.com" }).to_string();
	send(
		&ctx.app,
		"POST",
		"/api/auth/password-reset",
		Some(&body),
		None,
	)
	.await;
	ctx.deliver().await;

	let (_, list) = ctx.call("GET", "/api/email/outbox", Value::Null).await;
	assert_eq!(list[0]["kind"], "password_reset");
	assert_eq!(list[0]["status"], "failed");
	assert_eq!(list[0]["retryable"], false, "its link was scrubbed");
	let (status, _) = ctx
		.call(
			"POST",
			&format!(
				"/api/email/outbox/{}/retry",
				list[0]["id"].as_str().unwrap()
			),
			Value::Null,
		)
		.await;
	assert_eq!(status, 409);
}

//! Emailing a pay stub end to end (EMAIL-SPEC.md §8): request → outbox → email with the PDF.

use std::sync::Arc;

use crate::common::{app_with_db, insert_user, login, send};
use axum::Router;
use gr_logistics::email::{
	outbox::Outbox,
	render::Renderer,
	sender::{EmailSender, MemorySender},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

struct Ctx {
	app: Router,
	pool: PgPool,
	cookie: String,
	sender: Arc<MemorySender>,
	ana: Uuid,
	printer: Uuid,
}

impl Ctx {
	async fn new(pool: PgPool) -> Self {
		let app = app_with_db(pool.clone()).await;
		insert_user(&pool, "admin@example.com", "admin").await;
		let printer = insert_user(&pool, "printer@example.com", "employee").await;
		let cookie = login(&app, "admin@example.com").await;
		let mut ctx = Self {
			app,
			pool,
			cookie,
			sender: Arc::new(MemorySender::default()),
			ana: Uuid::nil(),
			printer,
		};
		let (_, ana) = ctx
			.call(
				"POST",
				"/api/employees",
				json!({ "firstName": "Ana", "lastName": "Díaz", "email": "ana@example.com",
					"payRatePercent": 42, "locale": "es" }),
			)
			.await;
		ctx.ana = ana["id"].as_str().unwrap().parse().unwrap();
		ctx
	}

	async fn call(&self, method: &str, uri: &str, body: Value) -> (u16, Value) {
		let body = body.to_string();
		let response = send(&self.app, method, uri, Some(&body), Some(&self.cookie)).await;
		(response.status.as_u16(), response.body)
	}

	/// A $1,000 week for Ana: 42% → $420.
	async fn paycheck(&self, complete: bool) -> Uuid {
		let (status, body) = self
			.call(
				"POST",
				"/api/paycheck-logs",
				json!({
					"employeeId": self.ana, "weekStart": "2026-10-04", "weekEnd": "2026-10-10",
					"payday": "2026-10-16", "complete": complete,
					"loads": [
						{ "storeName": "Store A", "loadDate": "2026-10-05", "pay": 600 },
						{ "storeName": "Store B", "loadDate": "2026-10-06", "pay": 350, "extraPay": 50 },
					],
				}),
			)
			.await;
		assert_eq!(status, 201, "{body:?}");
		body["id"].as_str().unwrap().parse().unwrap()
	}

	async fn email(&self, log: Uuid, user_ids: &[Uuid], employee_ids: &[Uuid]) -> (u16, Value) {
		self.call(
			"POST",
			&format!("/api/paycheck-logs/{log}/email"),
			json!({ "userIds": user_ids, "employeeIds": employee_ids }),
		)
		.await
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
}

#[sqlx::test(migrations = false)]
async fn each_recipient_gets_the_stub_in_their_language_with_the_pdf(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let log = ctx.paycheck(true).await;

	let (status, body) = ctx.email(log, &[ctx.printer], &[ctx.ana]).await;
	assert_eq!(status, 202, "{body:?}");
	assert_eq!(body["outboxIds"].as_array().unwrap().len(), 2);

	ctx.deliver().await;
	let mut sent = ctx.sender.sent();
	sent.sort_by(|a, b| a.to.cmp(&b.to));
	assert_eq!(sent.len(), 2, "one email per recipient");

	let (ana, printer) = (&sent[0], &sent[1]);
	assert_eq!(ana.to, "ana@example.com");
	assert_eq!(ana.subject, "Talón de pago: Ana Díaz · 4 oct – 10 oct 2026");
	assert!(ana.html.contains("$420.00"), "1000 × 42%");
	assert!(ana.html.contains("16 oct 2026"));

	assert_eq!(printer.to, "printer@example.com");
	assert_eq!(printer.subject, "Pay stub: Ana Díaz · Oct 4 – Oct 10, 2026");

	for email in [ana, printer] {
		assert_eq!(email.attachments.len(), 1);
		let pdf = &email.attachments[0];
		assert_eq!(pdf.content_type, "application/pdf");
		assert!(pdf.bytes.starts_with(b"%PDF"));
	}
	assert_eq!(
		ana.attachments[0].filename,
		"talon-de-pago-ana-diaz-2026-10-04.pdf"
	);
	assert_eq!(
		printer.attachments[0].filename,
		"pay-stub-ana-diaz-2026-10-04.pdf"
	);
}

#[sqlx::test(migrations = false)]
async fn only_completed_logs_can_be_emailed(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let draft = ctx.paycheck(false).await;

	let (status, body) = ctx.email(draft, &[ctx.printer], &[]).await;
	assert_eq!(status, 409);
	assert_eq!(body["code"], "PAYCHECK_NOT_COMPLETED");

	let (status, body) = ctx.email(Uuid::new_v4(), &[ctx.printer], &[]).await;
	assert_eq!(status, 404);
	assert_eq!(body["code"], "PAYCHECK_LOG_NOT_FOUND");
}

#[sqlx::test(migrations = false)]
async fn recipients_must_exist_and_be_given(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let log = ctx.paycheck(true).await;

	let (status, body) = ctx.email(log, &[], &[]).await;
	assert_eq!(status, 400);
	assert_eq!(body["code"], "NO_RECIPIENTS");

	let (status, body) = ctx.email(log, &[ctx.printer], &[Uuid::new_v4()]).await;
	assert_eq!(status, 400);
	assert_eq!(body["code"], "RECIPIENT_NOT_FOUND");

	let count: i64 = sqlx::query_scalar("SELECT count(*) FROM email_outbox")
		.fetch_one(&ctx.pool)
		.await
		.unwrap();
	assert_eq!(count, 0, "a rejected request queues nothing");
}

#[sqlx::test(migrations = false)]
async fn a_log_deleted_before_sending_fails_without_retrying(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let log = ctx.paycheck(true).await;
	ctx.email(log, &[ctx.printer], &[]).await;
	let (status, _) = ctx
		.call("DELETE", &format!("/api/paycheck-logs/{log}"), Value::Null)
		.await;
	assert_eq!(status, 204);

	ctx.deliver().await;

	assert!(ctx.sender.sent().is_empty());
	let (status, error): (String, String) =
		sqlx::query_as("SELECT status, last_error FROM email_outbox")
			.fetch_one(&ctx.pool)
			.await
			.unwrap();
	assert_eq!(status, "failed");
	assert_eq!(error, "paycheck log not found");
}

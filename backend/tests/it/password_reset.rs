//! Password reset end to end (EMAIL-SPEC.md §7): request → outbox → email → confirm.

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

const EMAIL: &str = "admin@example.com";
const NEW_PASSWORD: &str = "a-brand-new-password";

struct Ctx {
	app: Router,
	pool: PgPool,
	sender: Arc<MemorySender>,
}

impl Ctx {
	async fn new(pool: PgPool) -> Self {
		let app = app_with_db(pool.clone()).await;
		insert_user(&pool, EMAIL, "admin").await;
		Self {
			app,
			pool,
			sender: Arc::new(MemorySender::default()),
		}
	}

	async fn request(&self, email: &str, locale: &str) -> u16 {
		let body = json!({ "email": email, "locale": locale }).to_string();
		send(
			&self.app,
			"POST",
			"/api/auth/password-reset",
			Some(&body),
			None,
		)
		.await
		.status
		.as_u16()
	}

	async fn confirm(&self, token: &str, password: &str) -> (u16, Value) {
		let body = json!({ "token": token, "newPassword": password }).to_string();
		let response = send(
			&self.app,
			"POST",
			"/api/auth/password-reset/confirm",
			Some(&body),
			None,
		)
		.await;
		(response.status.as_u16(), response.body)
	}

	/// Runs the outbox once and returns the tokens from every link sent.
	async fn deliver(&self) -> Vec<String> {
		let before = self.sender.sent().len();
		Outbox::new(
			self.pool.clone(),
			self.sender.clone() as Arc<dyn EmailSender>,
			Arc::new(Renderer::new()),
		)
		.process_due()
		.await
		.unwrap();
		self.sender.sent()[before..]
			.iter()
			.map(|email| {
				let (_, rest) = email.text.split_once("/reset-password?token=").unwrap();
				rest.split_whitespace().next().unwrap().to_owned()
			})
			.collect()
	}

	async fn outbox_count(&self) -> i64 {
		sqlx::query_scalar("SELECT count(*) FROM email_outbox WHERE kind = 'password_reset'")
			.fetch_one(&self.pool)
			.await
			.unwrap()
	}

	async fn login_status(&self, password: &str) -> u16 {
		let body = json!({ "email": EMAIL, "password": password }).to_string();
		send(&self.app, "POST", "/api/auth/login", Some(&body), None)
			.await
			.status
			.as_u16()
	}
}

#[sqlx::test(migrations = false)]
async fn an_unknown_email_gets_the_same_202_and_no_email(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	assert_eq!(ctx.request("nobody@example.com", "en").await, 202);
	assert_eq!(ctx.outbox_count().await, 0);
}

#[sqlx::test(migrations = false)]
async fn the_emailed_link_resets_the_password_once_and_signs_out_everywhere(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let cookie = login(&ctx.app, EMAIL).await;

	assert_eq!(ctx.request(EMAIL, "es").await, 202);
	let tokens = ctx.deliver().await;
	assert_eq!(tokens.len(), 1);

	let email = &ctx.sender.sent()[0];
	assert_eq!(email.to, EMAIL);
	assert_eq!(email.subject, "Restablezca su contraseña de GR Logistics");
	// minijinja escapes `/` as `&#x2f;` in HTML (valid; mail clients decode it in href).
	assert!(
		email
			.html
			.replace("&#x2f;", "/")
			.contains("http://localhost:5173/reset-password?token="),
		"link built from CORS_ORIGIN"
	);

	let scrubbed: Option<Value> =
		sqlx::query_scalar("SELECT payload FROM email_outbox WHERE kind = 'password_reset'")
			.fetch_one(&ctx.pool)
			.await
			.unwrap();
	assert!(
		scrubbed.is_none(),
		"the live token doesn't stay in the outbox"
	);

	assert_eq!(ctx.confirm(&tokens[0], NEW_PASSWORD).await.0, 204);

	assert_eq!(ctx.login_status(NEW_PASSWORD).await, 200);
	assert_eq!(
		ctx.login_status(crate::common::BUN_FIXTURE_PASSWORD).await,
		401
	);
	let me = send(&ctx.app, "GET", "/api/auth/me", None, Some(&cookie)).await;
	assert_eq!(me.status, 401, "the old session was revoked");

	let (status, body) = ctx.confirm(&tokens[0], "yet-another-password").await;
	assert_eq!(status, 400, "a link works once");
	assert_eq!(body["code"], "RESET_TOKEN_INVALID");
}

#[sqlx::test(migrations = false)]
async fn an_expired_link_is_rejected(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	ctx.request(EMAIL, "en").await;
	let token = ctx.deliver().await.remove(0);

	sqlx::query("UPDATE password_reset_tokens SET expires_at = now() - interval '1 second'")
		.execute(&ctx.pool)
		.await
		.unwrap();

	let (status, body) = ctx.confirm(&token, NEW_PASSWORD).await;
	assert_eq!(status, 400);
	assert_eq!(body["code"], "RESET_TOKEN_INVALID");
	assert_eq!(
		ctx.login_status(crate::common::BUN_FIXTURE_PASSWORD).await,
		200
	);
}

#[sqlx::test(migrations = false)]
async fn a_new_request_retires_the_previous_link(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	// Delivered one at a time: a batch's send order isn't guaranteed.
	ctx.request(EMAIL, "en").await;
	let first = ctx.deliver().await.remove(0);
	ctx.request(EMAIL, "en").await;
	let second = ctx.deliver().await.remove(0);

	assert_eq!(ctx.confirm(&first, NEW_PASSWORD).await.0, 400);
	assert_eq!(ctx.confirm(&second, NEW_PASSWORD).await.0, 204);
}

#[sqlx::test(migrations = false)]
async fn more_than_three_requests_an_hour_send_nothing_more(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	for _ in 0..5 {
		assert_eq!(
			ctx.request(EMAIL, "en").await,
			202,
			"always the same answer"
		);
	}
	assert_eq!(ctx.outbox_count().await, 3);
}

#[sqlx::test(migrations = false)]
async fn bad_input_is_rejected(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	ctx.request(EMAIL, "en").await;
	let token = ctx.deliver().await.remove(0);

	let (status, body) = ctx.confirm(&token, "too-short").await;
	assert_eq!(status, 400);
	assert_eq!(body["code"], "VALIDATION_FAILED");
	assert_eq!(
		ctx.confirm(&token, &"x".repeat(129)).await.0,
		400,
		"over the 128 cap"
	);
	assert_eq!(
		ctx.confirm(&token, NEW_PASSWORD).await.0,
		204,
		"a rejected password doesn't use up the link"
	);

	assert_eq!(ctx.request("not-an-email", "en").await, 400);
}

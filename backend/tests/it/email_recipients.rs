//! Recipients & language (EMAIL-SPEC.md §6): recipient options, saved language, the monthly
//! report opt-in, and resolving picked ids to addresses.

use crate::common::{app_with_db, insert_user, login, send};
use axum::Router;
use gr_logistics::{
	locale::Locale,
	services::email_recipients::{self, Recipient},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

struct Ctx {
	app: Router,
	pool: PgPool,
	cookie: String,
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
			admin,
		}
	}

	async fn call(&self, method: &str, uri: &str, body: Option<Value>) -> (u16, Value) {
		let body = body.map(|b| b.to_string());
		let response = send(&self.app, method, uri, body.as_deref(), Some(&self.cookie)).await;
		(response.status.as_u16(), response.body)
	}

	async fn employee(&self, first: &str, email: Option<&str>, locale: Option<&str>) -> Uuid {
		let mut body = json!({ "firstName": first, "lastName": "Test" });
		if let Some(email) = email {
			body["email"] = json!(email);
		}
		if let Some(locale) = locale {
			body["locale"] = json!(locale);
		}
		let (status, body) = self.call("POST", "/api/employees", Some(body)).await;
		assert_eq!(status, 201, "{body:?}");
		body["id"].as_str().unwrap().parse().unwrap()
	}
}

#[sqlx::test(migrations = false)]
async fn options_list_users_and_employees_that_can_receive_email(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let ana = ctx.employee("Ana", Some("ana@example.com"), None).await;
	ctx.employee("NoEmail", None, None).await;
	let gone = ctx.employee("Gone", Some("gone@example.com"), None).await;
	ctx.call("DELETE", &format!("/api/employees/{gone}"), None)
		.await;

	let (status, body) = ctx.call("GET", "/api/email/recipient-options", None).await;
	assert_eq!(status, 200);
	assert_eq!(
		body["users"],
		json!([{ "id": ctx.admin, "email": "admin@example.com" }])
	);
	assert_eq!(
		body["employees"],
		json!([{ "id": ana, "firstName": "Ana", "lastName": "Test", "email": "ana@example.com" }]),
		"no employee without an email, none inactive"
	);
}

#[sqlx::test(migrations = false)]
async fn the_email_controller_is_admin_only(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	insert_user(&ctx.pool, "printer@example.com", "employee").await;
	let cookie = login(&ctx.app, "printer@example.com").await;

	for (method, uri) in [
		("GET", "/api/email/recipient-options"),
		("GET", "/api/email/monthly-report/recipients"),
	] {
		let response = send(&ctx.app, method, uri, None, Some(&cookie)).await;
		assert_eq!(response.status, 403, "{method} {uri}");
	}
}

#[sqlx::test(migrations = false)]
async fn a_user_saves_their_own_language(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let (_, me) = ctx.call("GET", "/api/auth/me", None).await;
	assert_eq!(me["locale"], "en", "the default");

	let (status, _) = ctx
		.call(
			"PUT",
			"/api/auth/me/locale",
			Some(json!({ "locale": "es" })),
		)
		.await;
	assert_eq!(status, 204);
	let (_, me) = ctx.call("GET", "/api/auth/me", None).await;
	assert_eq!(me["locale"], "es");

	let (status, body) = ctx
		.call(
			"PUT",
			"/api/auth/me/locale",
			Some(json!({ "locale": "fr" })),
		)
		.await;
	assert_eq!(status, 400);
	assert_eq!(body["code"], "VALIDATION_FAILED");

	// An email-only account (role `employee`) can save its language too.
	insert_user(&ctx.pool, "printer@example.com", "employee").await;
	let cookie = login(&ctx.app, "printer@example.com").await;
	let body = json!({ "locale": "es" }).to_string();
	let response = send(
		&ctx.app,
		"PUT",
		"/api/auth/me/locale",
		Some(&body),
		Some(&cookie),
	)
	.await;
	assert_eq!(response.status, 204);
}

#[sqlx::test(migrations = false)]
async fn a_user_saves_their_own_theme(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let (_, me) = ctx.call("GET", "/api/auth/me", None).await;
	assert_eq!(me["theme"], "default", "the default");

	let (status, _) = ctx
		.call(
			"PUT",
			"/api/auth/me/theme",
			Some(json!({ "theme": "dracula" })),
		)
		.await;
	assert_eq!(status, 204);
	let (_, me) = ctx.call("GET", "/api/auth/me", None).await;
	assert_eq!(me["theme"], "dracula");

	// Added by migration 0016, which widens 0015's CHECK.
	let (status, _) = ctx
		.call(
			"PUT",
			"/api/auth/me/theme",
			Some(json!({ "theme": "stormy" })),
		)
		.await;
	assert_eq!(status, 204);

	let (status, body) = ctx
		.call(
			"PUT",
			"/api/auth/me/theme",
			Some(json!({ "theme": "neon" })),
		)
		.await;
	assert_eq!(status, 400);
	assert_eq!(body["code"], "VALIDATION_FAILED");
}

#[sqlx::test(migrations = false)]
async fn employees_have_a_language(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let id = ctx.employee("Ana", None, None).await;
	let (_, list) = ctx.call("GET", "/api/employees", None).await;
	assert_eq!(list[0]["locale"], "en", "the default");

	let (status, body) = ctx
		.call(
			"PUT",
			&format!("/api/employees/{id}"),
			Some(json!({ "locale": "es" })),
		)
		.await;
	assert_eq!(status, 200);
	assert_eq!(body["locale"], "es");

	let (status, _) = ctx
		.call(
			"POST",
			"/api/employees",
			Some(json!({ "firstName": "A", "lastName": "B", "locale": "fr" })),
		)
		.await;
	assert_eq!(status, 400);
}

#[sqlx::test(migrations = false)]
async fn monthly_report_recipients_are_replaced_as_a_set(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let printer = insert_user(&ctx.pool, "printer@example.com", "employee").await;

	let (status, body) = ctx
		.call(
			"PUT",
			"/api/email/monthly-report/recipients",
			Some(json!({ "userIds": [ctx.admin, printer] })),
		)
		.await;
	assert_eq!(status, 200);
	assert_eq!(body["userIds"].as_array().unwrap().len(), 2);

	let (_, body) = ctx
		.call(
			"PUT",
			"/api/email/monthly-report/recipients",
			Some(json!({ "userIds": [printer] })),
		)
		.await;
	assert_eq!(body["userIds"], json!([printer]), "the admin was taken off");

	let (status, body) = ctx
		.call(
			"PUT",
			"/api/email/monthly-report/recipients",
			Some(json!({ "userIds": [ctx.admin, Uuid::new_v4()] })),
		)
		.await;
	assert_eq!(status, 400);
	assert_eq!(body["code"], "RECIPIENT_NOT_FOUND");

	let (_, body) = ctx
		.call("GET", "/api/email/monthly-report/recipients", None)
		.await;
	assert_eq!(
		body["userIds"],
		json!([printer]),
		"a rejected update changes nothing"
	);
}

#[sqlx::test(migrations = false)]
async fn picked_ids_resolve_to_addresses_in_each_recipients_language(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let ana = ctx
		.employee("Ana", Some("ana@example.com"), Some("es"))
		.await;
	let same_as_admin = ctx.employee("Admin", Some("ADMIN@example.com"), None).await;
	let no_email = ctx.employee("NoEmail", None, None).await;
	let gone = ctx.employee("Gone", Some("gone@example.com"), None).await;
	ctx.call("DELETE", &format!("/api/employees/{gone}"), None)
		.await;

	let mut conn = ctx.pool.acquire().await.unwrap();
	let resolved = email_recipients::resolve(&mut conn, &[ctx.admin], &[ana, same_as_admin])
		.await
		.unwrap()
		.unwrap();
	assert_eq!(
		resolved,
		[
			Recipient {
				email: "admin@example.com".into(),
				locale: Locale::En
			},
			Recipient {
				email: "ana@example.com".into(),
				locale: Locale::Es
			},
		],
		"one email per address, whatever the case"
	);

	for (users, employees) in [
		(vec![Uuid::new_v4()], vec![]),
		(vec![], vec![no_email]),
		(vec![], vec![gone]),
	] {
		assert!(
			email_recipients::resolve(&mut conn, &users, &employees)
				.await
				.unwrap()
				.is_none(),
			"{users:?} {employees:?}"
		);
	}
}

//! Runs against real Postgres (`#[sqlx::test]`, DATABASE_URL from `varlock run` / .env.schema).

mod common;

use common::{app_with_db, insert_user, login, send};
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test(migrations = false)]
async fn login_with_a_bun_written_hash_sets_the_session_cookie(pool: PgPool) {
	let app = app_with_db(pool.clone()).await;
	insert_user(&pool, "admin@example.com", "admin").await;

	let body = r#"{"email":"admin@example.com","password":"bun-fixture-password"}"#;
	let response = send(&app, "POST", "/api/auth/login", Some(body), None).await;

	assert_eq!(response.status, 200);
	assert_eq!(response.body["email"], "admin@example.com");
	assert_eq!(response.body["role"], "admin");

	// Same attributes as the Bun backend's cookie (Secure only in production).
	let cookie = response.headers["set-cookie"].to_str().unwrap();
	assert!(cookie.starts_with("gr_session="), "{cookie}");
	for attribute in ["HttpOnly", "SameSite=Lax", "Path=/", "Expires="] {
		assert!(cookie.contains(attribute), "missing {attribute}: {cookie}");
	}
	assert!(!cookie.contains("Secure"), "{cookie}");
}

#[sqlx::test(migrations = false)]
async fn wrong_password_and_unknown_email_get_the_same_401(pool: PgPool) {
	let app = app_with_db(pool.clone()).await;
	insert_user(&pool, "admin@example.com", "admin").await;

	for body in [
		r#"{"email":"admin@example.com","password":"wrong"}"#,
		r#"{"email":"nobody@example.com","password":"bun-fixture-password"}"#,
	] {
		let response = send(&app, "POST", "/api/auth/login", Some(body), None).await;
		assert_eq!(response.status, 401, "{body}");
		assert_eq!(
			response.body,
			json!({ "error": "Invalid email or password", "code": "INVALID_CREDENTIALS" })
		);
	}
}

/// Bad input is 400 with the `{ "error" }` shape (Bun returned 500).
#[sqlx::test(migrations = false)]
async fn invalid_login_bodies_are_400(pool: PgPool) {
	let app = app_with_db(pool).await;

	for body in [
		r#"{"email":"not-an-email","password":"x"}"#,
		r#"{"email":"a@example.com","password":""}"#,
		r#"{"email":"a@example.com"}"#,
		r#"{"email":"a@example.com","password":42}"#,
		"not json",
	] {
		let response = send(&app, "POST", "/api/auth/login", Some(body), None).await;
		assert_eq!(response.status, 400, "{body}");
		assert!(
			response.body["error"].is_string(),
			"{body}: {:?}",
			response.body
		);
		assert!(
			["INVALID_BODY", "VALIDATION_FAILED"]
				.contains(&response.body["code"].as_str().unwrap()),
			"{body}: {:?}",
			response.body
		);
	}
}

#[sqlx::test(migrations = false)]
async fn me_works_until_logout(pool: PgPool) {
	let app = app_with_db(pool.clone()).await;
	insert_user(&pool, "admin@example.com", "admin").await;
	let cookie = login(&app, "admin@example.com").await;

	let me = send(&app, "GET", "/api/auth/me", None, Some(&cookie)).await;
	assert_eq!(me.status, 200);
	assert_eq!(me.body["email"], "admin@example.com");

	let logout = send(&app, "POST", "/api/auth/logout", None, Some(&cookie)).await;
	assert_eq!(logout.status, 204);
	let cleared = logout.headers["set-cookie"].to_str().unwrap();
	assert!(
		cleared.contains("gr_session=") && cleared.contains("Max-Age=0"),
		"{cleared}"
	);

	let after = send(&app, "GET", "/api/auth/me", None, Some(&cookie)).await;
	assert_eq!(after.status, 401);
}

#[sqlx::test(migrations = false)]
async fn logout_without_a_session_is_still_204(pool: PgPool) {
	let app = app_with_db(pool).await;
	let response = send(&app, "POST", "/api/auth/logout", None, None).await;
	assert_eq!(response.status, 204);
}

/// A session row created outside this backend (e.g. by the original Bun backend) is accepted.
#[sqlx::test(migrations = false)]
async fn accepts_a_session_created_outside_the_rust_backend(pool: PgPool) {
	let app = app_with_db(pool.clone()).await;
	let user_id = insert_user(&pool, "admin@example.com", "admin").await;

	let session_id: uuid::Uuid = sqlx::query_scalar(
		"INSERT INTO sessions (user_id, expires_at) VALUES ($1, now() + interval '1 hour')
         RETURNING session_id",
	)
	.bind(user_id)
	.fetch_one(&pool)
	.await
	.unwrap();

	let cookie = format!("gr_session={session_id}");
	let response = send(&app, "GET", "/api/auth/me", None, Some(&cookie)).await;
	assert_eq!(response.status, 200);
}

#[sqlx::test(migrations = false)]
async fn expired_and_malformed_sessions_are_401(pool: PgPool) {
	let app = app_with_db(pool.clone()).await;
	let user_id = insert_user(&pool, "admin@example.com", "admin").await;

	let expired: uuid::Uuid = sqlx::query_scalar(
		"INSERT INTO sessions (user_id, expires_at) VALUES ($1, now() - interval '1 minute')
         RETURNING session_id",
	)
	.bind(user_id)
	.fetch_one(&pool)
	.await
	.unwrap();

	for cookie in [
		format!("gr_session={expired}"),
		format!("gr_session={}", uuid::Uuid::new_v4()),
		"gr_session=not-a-uuid".to_owned(),
	] {
		let response = send(&app, "GET", "/api/auth/me", None, Some(&cookie)).await;
		assert_eq!(response.status, 401, "{cookie}");
	}
}

/// Decision #4: data routes check the role explicitly, reads included.
#[sqlx::test(migrations = false)]
async fn non_admin_role_is_403_on_data_routes(pool: PgPool) {
	let app = app_with_db(pool.clone()).await;
	insert_user(&pool, "driver@example.com", "employee").await;
	let cookie = login(&app, "driver@example.com").await;

	assert_eq!(
		send(&app, "GET", "/api/auth/me", None, Some(&cookie))
			.await
			.status,
		200
	);

	for (method, uri) in [
		("GET", "/api/employees"),
		("GET", "/api/paycheck-logs"),
		("GET", "/api/dashboard/summary"),
	] {
		let response = send(&app, method, uri, None, Some(&cookie)).await;
		assert_eq!(response.status, 403, "{method} {uri}");
	}
}

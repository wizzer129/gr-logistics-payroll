//! Runs against real Postgres (`#[sqlx::test]`, DATABASE_URL from `varlock run` / .env.schema).

mod common;

use axum::Router;
use common::{app_with_db, insert_user, login, send};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

async fn admin_app(pool: &PgPool) -> (Router, String, Uuid) {
	let app = app_with_db(pool.clone()).await;
	let user_id = insert_user(pool, "admin@example.com", "admin").await;
	let cookie = login(&app, "admin@example.com").await;
	(app, cookie, user_id)
}

/// Numbers come back exact (e.g. `42.00`), so compare numerically.
fn num(value: &Value) -> f64 {
	value
		.as_f64()
		.unwrap_or_else(|| panic!("not a JSON number: {value}"))
}

async fn audit_rows(pool: &PgPool, entity_id: &str) -> Vec<(String, Value)> {
	sqlx::query_as::<_, (String, sqlx::types::Json<Value>)>(
		"SELECT action, diff FROM audit_log WHERE entity_id = $1::uuid ORDER BY created_at",
	)
	.bind(entity_id)
	.fetch_all(pool)
	.await
	.unwrap()
	.into_iter()
	.map(|(action, diff)| (action, diff.0))
	.collect()
}

/// Exactly what frontend/src/components/employees/EmployeeForm.vue sends when the email
/// field is left blank. Bun rejected this (Zod `.email()` on "") with a 500.
#[sqlx::test(migrations = false)]
async fn creates_from_the_frontend_form_payload(pool: PgPool) {
	let (app, cookie, admin_id) = admin_app(&pool).await;

	let body = r#"{"firstName":"Ana","lastName":"Diaz","email":"","payRatePercent":42}"#;
	let response = send(&app, "POST", "/api/employees", Some(body), Some(&cookie)).await;

	assert_eq!(response.status, 201, "{:?}", response.body);
	let employee = &response.body;
	assert_eq!(employee["firstName"], "Ana");
	assert_eq!(employee["lastName"], "Diaz");
	assert_eq!(employee["email"], Value::Null);
	assert_eq!(employee["active"], true);
	assert_eq!(num(&employee["payRatePercent"]), 42.0);

	let id = employee["id"].as_str().unwrap();
	let audit = audit_rows(&pool, id).await;
	assert_eq!(audit.len(), 1);
	assert_eq!(audit[0].0, "create");
	assert_eq!(audit[0].1["firstName"], "Ana");
	assert!(
		audit[0].1.get("email").is_none(),
		"blank email isn't recorded"
	);

	let created_by: Uuid =
		sqlx::query_scalar("SELECT created_by FROM employees WHERE id = $1::uuid")
			.bind(id)
			.fetch_one(&pool)
			.await
			.unwrap();
	assert_eq!(created_by, admin_id);
}

#[sqlx::test(migrations = false)]
async fn pay_rate_defaults_to_42_and_is_stored_exactly(pool: PgPool) {
	let (app, cookie, _) = admin_app(&pool).await;

	let default = send(
		&app,
		"POST",
		"/api/employees",
		Some(r#"{"firstName":"A","lastName":"B"}"#),
		Some(&cookie),
	)
	.await;
	assert_eq!(num(&default.body["payRatePercent"]), 42.0);

	let custom = send(
		&app,
		"POST",
		"/api/employees",
		Some(r#"{"firstName":"C","lastName":"D","payRatePercent":40.1,"dob":"1990-05-17"}"#),
		Some(&cookie),
	)
	.await;
	assert_eq!(custom.status, 201);
	assert_eq!(custom.body["dob"], "1990-05-17", "date-only, no time part");

	let stored: String =
		sqlx::query_scalar("SELECT pay_rate_percent::text FROM employees WHERE id = $1::uuid")
			.bind(custom.body["id"].as_str().unwrap())
			.fetch_one(&pool)
			.await
			.unwrap();
	assert_eq!(stored, "40.10");
}

#[sqlx::test(migrations = false)]
async fn invalid_employee_bodies_are_400(pool: PgPool) {
	let (app, cookie, _) = admin_app(&pool).await;

	for body in [
		r#"{"firstName":"","lastName":"B"}"#,
		r#"{"lastName":"B"}"#,
		r#"{"firstName":"A","lastName":"B","email":"not-an-email"}"#,
		r#"{"firstName":"A","lastName":"B","dob":"2026-13-01"}"#,
		r#"{"firstName":"A","lastName":"B","payRatePercent":0}"#,
		r#"{"firstName":"A","lastName":"B","payRatePercent":101}"#,
		r#"{"firstName":"A","lastName":"B","payRatePercent":40.555}"#,
		r#"{"firstName":"A","lastName":"B","payRatePercent":"42"}"#,
	] {
		let response = send(&app, "POST", "/api/employees", Some(body), Some(&cookie)).await;
		assert_eq!(response.status, 400, "{body}: {:?}", response.body);
		assert!(response.body["error"].is_string());
	}

	let count: i64 = sqlx::query_scalar("SELECT count(*) FROM employees")
		.fetch_one(&pool)
		.await
		.unwrap();
	assert_eq!(count, 0);
}

#[sqlx::test(migrations = false)]
async fn list_is_active_only_sorted_by_last_then_first_name(pool: PgPool) {
	let (app, cookie, _) = admin_app(&pool).await;

	let mut ids = Vec::new();
	for (first, last) in [
		("Zed", "Alpha"),
		("Amy", "Beta"),
		("Bob", "Alpha"),
		("Gone", "Aaa"),
	] {
		let body = format!(r#"{{"firstName":"{first}","lastName":"{last}"}}"#);
		let response = send(&app, "POST", "/api/employees", Some(&body), Some(&cookie)).await;
		ids.push(response.body["id"].as_str().unwrap().to_owned());
	}
	let deleted = format!("/api/employees/{}", ids[3]);
	send(&app, "DELETE", &deleted, None, Some(&cookie)).await;

	let response = send(&app, "GET", "/api/employees", None, Some(&cookie)).await;
	assert_eq!(response.status, 200);
	let names: Vec<_> = response
		.body
		.as_array()
		.unwrap()
		.iter()
		.map(|e| {
			format!(
				"{} {}",
				e["firstName"].as_str().unwrap(),
				e["lastName"].as_str().unwrap()
			)
		})
		.collect();
	assert_eq!(names, ["Bob Alpha", "Zed Alpha", "Amy Beta"]);
}

#[sqlx::test(migrations = false)]
async fn partial_update_keeps_omitted_fields(pool: PgPool) {
	let (app, cookie, _) = admin_app(&pool).await;
	let created = send(
		&app,
		"POST",
		"/api/employees",
		Some(r#"{"firstName":"Ana","lastName":"Diaz","email":"ana@example.com"}"#),
		Some(&cookie),
	)
	.await;
	let uri = format!("/api/employees/{}", created.body["id"].as_str().unwrap());

	let response = send(
		&app,
		"PUT",
		&uri,
		Some(r#"{"payRatePercent":40}"#),
		Some(&cookie),
	)
	.await;
	assert_eq!(response.status, 200);
	assert_eq!(num(&response.body["payRatePercent"]), 40.0);
	assert_eq!(response.body["firstName"], "Ana");
	assert_eq!(response.body["email"], "ana@example.com");

	let blank_name = send(
		&app,
		"PUT",
		&uri,
		Some(r#"{"firstName":""}"#),
		Some(&cookie),
	)
	.await;
	assert_eq!(blank_name.status, 400);

	let audit = audit_rows(&pool, created.body["id"].as_str().unwrap()).await;
	assert_eq!(audit.len(), 2, "create + one successful update");
	assert_eq!(audit[1].0, "update");
	assert_eq!(num(&audit[1].1["payRatePercent"]), 40.0);
}

#[sqlx::test(migrations = false)]
async fn delete_is_soft_and_audited(pool: PgPool) {
	let (app, cookie, _) = admin_app(&pool).await;
	let created = send(
		&app,
		"POST",
		"/api/employees",
		Some(r#"{"firstName":"Ana","lastName":"Diaz"}"#),
		Some(&cookie),
	)
	.await;
	let id = created.body["id"].as_str().unwrap();

	let response = send(
		&app,
		"DELETE",
		&format!("/api/employees/{id}"),
		None,
		Some(&cookie),
	)
	.await;
	assert_eq!(response.status, 204);

	let active: bool = sqlx::query_scalar("SELECT active FROM employees WHERE id = $1::uuid")
		.bind(id)
		.fetch_one(&pool)
		.await
		.unwrap();
	assert!(!active, "row still exists, just inactive");

	let audit = audit_rows(&pool, id).await;
	assert_eq!(audit.last().unwrap().0, "delete");
	assert_eq!(audit.last().unwrap().1, serde_json::json!({}));
}

#[sqlx::test(migrations = false)]
async fn unknown_and_malformed_ids(pool: PgPool) {
	let (app, cookie, _) = admin_app(&pool).await;
	let missing = format!("/api/employees/{}", Uuid::new_v4());

	let update = send(
		&app,
		"PUT",
		&missing,
		Some(r#"{"firstName":"X"}"#),
		Some(&cookie),
	)
	.await;
	assert_eq!(update.status, 404);
	assert_eq!(update.body["error"], "Employee not found");
	assert_eq!(update.body["code"], "EMPLOYEE_NOT_FOUND");
	assert_eq!(
		send(&app, "DELETE", &missing, None, Some(&cookie))
			.await
			.status,
		404
	);

	let malformed = send(
		&app,
		"DELETE",
		"/api/employees/not-a-uuid",
		None,
		Some(&cookie),
	)
	.await;
	assert_eq!(malformed.status, 400);
	assert_eq!(malformed.body["error"], "Invalid id");
	assert_eq!(malformed.body["code"], "INVALID_ID");

	let audit_count: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log")
		.fetch_one(&pool)
		.await
		.unwrap();
	assert_eq!(audit_count, 0, "failed writes leave no audit rows");
}

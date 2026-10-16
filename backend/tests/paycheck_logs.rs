//! Runs against real Postgres (`#[sqlx::test]`, DATABASE_URL from `varlock run` / .env.schema).

mod common;

use axum::Router;
use common::{TestResponse, app_with_db, insert_user, login, send};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

struct Ctx {
	app: Router,
	cookie: String,
	pool: PgPool,
}

impl Ctx {
	async fn new(pool: PgPool) -> Self {
		let app = app_with_db(pool.clone()).await;
		insert_user(&pool, "admin@example.com", "admin").await;
		let cookie = login(&app, "admin@example.com").await;
		Self { app, cookie, pool }
	}

	async fn req(&self, method: &str, uri: &str, body: Option<&Value>) -> TestResponse {
		let body = body.map(Value::to_string);
		send(&self.app, method, uri, body.as_deref(), Some(&self.cookie)).await
	}

	async fn employee(&self, pay_rate: Value) -> String {
		let body = json!({ "firstName": "Ana", "lastName": "Diaz", "payRatePercent": pay_rate });
		let response = self.req("POST", "/api/employees", Some(&body)).await;
		response.body["id"].as_str().unwrap().to_owned()
	}

	async fn count(&self, sql: &'static str) -> i64 {
		sqlx::query_scalar(sql).fetch_one(&self.pool).await.unwrap()
	}
}

fn num(value: &Value) -> f64 {
	value
		.as_f64()
		.unwrap_or_else(|| panic!("not a JSON number: {value}"))
}

/// The shape frontend/src/store/payment-processing-store.ts submits.
fn log_body(employee_id: &str, week_start: &str, loads: Value) -> Value {
	json!({
		"employeeId": employee_id,
		"weekStart": week_start,
		"weekEnd": "2026-10-10",
		"payday": "2026-10-16",
		"loads": loads,
	})
}

fn one_load(pay: Value) -> Value {
	json!([{ "storeName": "Walmart 12", "loadDate": "2026-10-05", "pay": pay }])
}

#[sqlx::test(migrations = false)]
async fn creates_a_draft_with_exact_decimal_totals(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let employee = ctx.employee(json!(42)).await;

	let loads = json!([
		{ "storeName": "Walmart 12", "loadDate": "2026-10-05", "pay": 100.25, "extraPay": 10 },
		{ "storeName": "Target 3", "loadDate": "2026-10-06", "pay": 50 },
		{ "storeName": "walmart 12", "loadDate": "2026-10-07", "pay": 0.1, "extraPay": 0.2 },
	]);
	let response = ctx
		.req(
			"POST",
			"/api/paycheck-logs",
			Some(&log_body(&employee, "2026-10-04", loads)),
		)
		.await;

	assert_eq!(response.status, 201, "{:?}", response.body);
	let log = &response.body;
	assert_eq!(log["status"], "draft");
	assert_eq!(log["employeeId"], employee);
	assert_eq!(log["weekStart"], "2026-10-04");
	assert_eq!(log["weekEnd"], "2026-10-10");
	assert_eq!(log["payday"], "2026-10-16");
	// 110.25 + 50 + 0.3 = 160.55 exactly (floats would drift on 0.1 + 0.2).
	assert_eq!(log["summaryTotal"].to_string(), "160.55");
	// 160.55 × 42% = 67.431 → 67
	assert_eq!(num(&log["employeeCut"]), 67.0);

	assert_eq!(ctx.count("SELECT count(*) FROM employee_loads").await, 3);
	assert_eq!(
		ctx.count("SELECT count(*) FROM stores").await,
		2,
		"walmart 12 reused"
	);
	assert_eq!(
		ctx.count(
			"SELECT count(*) FROM audit_log WHERE entity = 'employee_paycheck_log' AND action = 'create'"
		)
		.await,
		1
	);
}

#[sqlx::test(migrations = false)]
async fn cut_rounds_half_up_at_the_employees_rate(pool: PgPool) {
	let ctx = Ctx::new(pool).await;

	// 125 × 42% = 52.50 → 53 (banker's rounding would give 52).
	let standard = ctx.employee(json!(42)).await;
	let response = ctx
		.req(
			"POST",
			"/api/paycheck-logs",
			Some(&log_body(&standard, "2026-10-04", one_load(json!(125)))),
		)
		.await;
	assert_eq!(num(&response.body["employeeCut"]), 53.0);

	// 101.20 × 40% = 40.48 → 40, at this employee's own rate.
	let forty = ctx.employee(json!(40)).await;
	let response = ctx
		.req(
			"POST",
			"/api/paycheck-logs",
			Some(&log_body(&forty, "2026-10-04", one_load(json!(101.20)))),
		)
		.await;
	assert_eq!(num(&response.body["employeeCut"]), 40.0);
}

#[sqlx::test(migrations = false)]
async fn invalid_bodies_are_400_and_write_nothing(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let employee = ctx.employee(json!(42)).await;
	let load = |overrides: Value| {
		let mut load = json!({ "storeName": "Walmart 12", "loadDate": "2026-10-05", "pay": 100 });
		load.as_object_mut()
			.unwrap()
			.extend(overrides.as_object().unwrap().clone());
		json!([load])
	};

	let cases = [
		log_body(&employee, "2026-10-04", json!([])),
		log_body(&employee, "2026-10-04", load(json!({ "pay": -1 }))),
		log_body(&employee, "2026-10-04", load(json!({ "pay": 1.005 }))),
		log_body(&employee, "2026-10-04", load(json!({ "pay": "100" }))),
		log_body(&employee, "2026-10-04", load(json!({ "extraPay": -5 }))),
		log_body(&employee, "2026-10-04", load(json!({ "storeName": "   " }))),
		log_body(
			&employee,
			"2026-10-04",
			load(json!({ "loadDate": "2026-10-05T00:00:00.000Z" })),
		),
		log_body(&employee, "2026-10-11", one_load(json!(100))), // weekEnd before weekStart
		log_body(
			&employee,
			"2026-10-04",
			json!([
				{ "storeName": "A", "loadDate": "2026-10-05", "pay": 99999999.99 },
				{ "storeName": "B", "loadDate": "2026-10-06", "pay": 1 },
			]),
		),
		log_body("not-a-uuid", "2026-10-04", one_load(json!(100))),
		json!({ "weekStart": "2026-10-04", "weekEnd": "2026-10-10", "payday": "2026-10-16", "loads": one_load(json!(1)) }),
	];

	for body in &cases {
		let response = ctx.req("POST", "/api/paycheck-logs", Some(body)).await;
		assert_eq!(response.status, 400, "{body}: {:?}", response.body);
		assert!(response.body["error"].is_string());
	}

	assert_eq!(
		ctx.count("SELECT count(*) FROM employee_paycheck_logs")
			.await,
		0
	);
	assert_eq!(ctx.count("SELECT count(*) FROM stores").await, 0);
}

#[sqlx::test(migrations = false)]
async fn unknown_employee_is_404_and_duplicate_week_is_409(pool: PgPool) {
	let ctx = Ctx::new(pool).await;

	let missing = log_body(
		&Uuid::new_v4().to_string(),
		"2026-10-04",
		one_load(json!(1)),
	);
	let response = ctx.req("POST", "/api/paycheck-logs", Some(&missing)).await;
	assert_eq!(response.status, 404);
	assert_eq!(response.body["error"], "Employee not found");
	assert_eq!(response.body["code"], "EMPLOYEE_NOT_FOUND");

	let employee = ctx.employee(json!(42)).await;
	let body = log_body(&employee, "2026-10-04", one_load(json!(100)));
	assert_eq!(
		ctx.req("POST", "/api/paycheck-logs", Some(&body))
			.await
			.status,
		201
	);

	// Bun returned 500 here (UNIQUE (employee_id, week_start)).
	let duplicate = ctx.req("POST", "/api/paycheck-logs", Some(&body)).await;
	assert_eq!(duplicate.status, 409, "{:?}", duplicate.body);
	assert_eq!(duplicate.body["code"], "DUPLICATE_WEEK");
	assert_eq!(
		ctx.count("SELECT count(*) FROM employee_paycheck_logs")
			.await,
		1
	);
	assert_eq!(ctx.count("SELECT count(*) FROM employee_loads").await, 1);
}

/// What frontend/src/views/EditPaymentView.vue does: GET the loads, send them straight back
/// (each still carrying its `id`) with new values.
#[sqlx::test(migrations = false)]
async fn edit_round_trips_loads_and_recomputes(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let employee = ctx.employee(json!(42)).await;
	let created = ctx
		.req(
			"POST",
			"/api/paycheck-logs",
			Some(&log_body(&employee, "2026-10-04", one_load(json!(100)))),
		)
		.await;
	let id = created.body["id"].as_str().unwrap();

	let loads = ctx
		.req("GET", &format!("/api/paycheck-logs/{id}/loads"), None)
		.await;
	assert_eq!(loads.status, 200);
	let mut loads = loads.body;
	assert_eq!(
		loads[0]["loadDate"], "2026-10-05",
		"date-only, so it round-trips"
	);
	assert_eq!(loads[0]["storeName"], "Walmart 12");
	assert_eq!(num(&loads[0]["extraPay"]), 0.0);

	loads[0]["pay"] = json!(200);
	loads.as_array_mut().unwrap().push(
		json!({ "storeName": "Target 3", "loadDate": "2026-10-08", "pay": 50, "extraPay": 0 }),
	);

	let edit = json!({ "weekStart": "2026-10-04", "weekEnd": "2026-10-10", "payday": "2026-10-17", "loads": loads });
	let response = ctx
		.req("PUT", &format!("/api/paycheck-logs/{id}"), Some(&edit))
		.await;

	assert_eq!(response.status, 200, "{:?}", response.body);
	assert_eq!(num(&response.body["summaryTotal"]), 250.0);
	assert_eq!(num(&response.body["employeeCut"]), 105.0);
	assert_eq!(response.body["payday"], "2026-10-17");
	assert_eq!(
		response.body["employeeId"], employee,
		"employee is immutable"
	);

	let after = ctx
		.req("GET", &format!("/api/paycheck-logs/{id}/loads"), None)
		.await;
	assert_eq!(
		after.body.as_array().unwrap().len(),
		2,
		"loads replaced, not appended"
	);

	let diff: sqlx::types::Json<Value> = sqlx::query_scalar(
		"SELECT diff FROM audit_log WHERE entity_id = $1::uuid AND action = 'update'",
	)
	.bind(id)
	.fetch_one(&ctx.pool)
	.await
	.unwrap();
	assert_eq!(diff.0["loads"].as_array().unwrap().len(), 2);
	assert_eq!(diff.0["payday"], "2026-10-17");
}

#[sqlx::test(migrations = false)]
async fn completed_totals_survive_a_pay_rate_change(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let employee = ctx.employee(json!(42)).await;
	let created = ctx
		.req(
			"POST",
			"/api/paycheck-logs",
			Some(&log_body(&employee, "2026-10-04", one_load(json!(1000)))),
		)
		.await;
	let id = created.body["id"].as_str().unwrap();

	let completed = ctx
		.req("POST", &format!("/api/paycheck-logs/{id}/complete"), None)
		.await;
	assert_eq!(completed.status, 200);
	assert_eq!(completed.body["status"], "completed");
	assert_eq!(num(&completed.body["employeeCut"]), 420.0);

	let rate = json!({ "payRatePercent": 40 });
	ctx.req("PUT", &format!("/api/employees/{employee}"), Some(&rate))
		.await;

	let list = ctx.req("GET", "/api/paycheck-logs", None).await;
	assert_eq!(list.body[0]["status"], "completed");
	assert_eq!(
		num(&list.body[0]["employeeCut"]),
		420.0,
		"snapshot, not recomputed at 40%"
	);

	let diff: sqlx::types::Json<Value> = sqlx::query_scalar(
		"SELECT diff FROM audit_log WHERE entity_id = $1::uuid AND action = 'update'",
	)
	.bind(id)
	.fetch_one(&ctx.pool)
	.await
	.unwrap();
	assert_eq!(diff.0, json!({ "status": "completed" }));
}

#[sqlx::test(migrations = false)]
async fn delete_cascades_loads_and_is_audited(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let employee = ctx.employee(json!(42)).await;
	let created = ctx
		.req(
			"POST",
			"/api/paycheck-logs",
			Some(&log_body(&employee, "2026-10-04", one_load(json!(100)))),
		)
		.await;
	let id = created.body["id"].as_str().unwrap();

	assert_eq!(
		ctx.req("DELETE", &format!("/api/paycheck-logs/{id}"), None)
			.await
			.status,
		204
	);
	assert_eq!(
		ctx.count("SELECT count(*) FROM employee_paycheck_logs")
			.await,
		0
	);
	assert_eq!(ctx.count("SELECT count(*) FROM employee_loads").await, 0);
	assert_eq!(
		ctx.count(
			"SELECT count(*) FROM audit_log WHERE action = 'delete' AND entity = 'employee_paycheck_log'"
		)
		.await,
		1
	);

	let missing = format!("/api/paycheck-logs/{}", Uuid::new_v4());
	assert_eq!(ctx.req("DELETE", &missing, None).await.status, 404);
	assert_eq!(
		ctx.req("POST", &format!("{missing}/complete"), None)
			.await
			.status,
		404
	);
	let update = json!({ "weekStart": "2026-10-04", "weekEnd": "2026-10-10", "payday": "2026-10-16", "loads": one_load(json!(1)) });
	assert_eq!(ctx.req("PUT", &missing, Some(&update)).await.status, 404);

	let loads = ctx.req("GET", &format!("{missing}/loads"), None).await;
	assert_eq!(loads.status, 200);
	assert_eq!(loads.body, json!([]));
}

#[sqlx::test(migrations = false)]
async fn list_filters_and_orders_newest_first(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let ana = ctx.employee(json!(42)).await;
	let other = ctx.employee(json!(42)).await;

	for (employee, start, end) in [
		(&ana, "2026-09-20", "2026-09-26"),
		(&ana, "2026-10-04", "2026-10-10"),
		(&other, "2026-09-27", "2026-10-03"),
	] {
		let body = json!({
			"employeeId": employee, "weekStart": start, "weekEnd": end, "payday": end,
			"loads": [{ "storeName": "A", "loadDate": start, "pay": 10 }],
		});
		assert_eq!(
			ctx.req("POST", "/api/paycheck-logs", Some(&body))
				.await
				.status,
			201
		);
	}

	let weeks = |response: &TestResponse| -> Vec<String> {
		response
			.body
			.as_array()
			.unwrap()
			.iter()
			.map(|l| l["weekStart"].as_str().unwrap().to_owned())
			.collect()
	};

	let all = ctx.req("GET", "/api/paycheck-logs", None).await;
	assert_eq!(weeks(&all), ["2026-10-04", "2026-09-27", "2026-09-20"]);
	assert_eq!(all.body[0]["firstName"], "Ana");
	assert_eq!(all.body[0]["createdByEmail"], "admin@example.com");
	assert!(all.body[0]["createdAt"].is_string());

	let by_employee = ctx
		.req("GET", &format!("/api/paycheck-logs?employeeId={ana}"), None)
		.await;
	assert_eq!(weeks(&by_employee), ["2026-10-04", "2026-09-20"]);

	let ranged = ctx
		.req(
			"GET",
			"/api/paycheck-logs?from=2026-09-27&to=2026-10-03",
			None,
		)
		.await;
	assert_eq!(weeks(&ranged), ["2026-09-27"]);

	let bad = ctx
		.req("GET", "/api/paycheck-logs?employeeId=abc", None)
		.await;
	assert_eq!(bad.status, 400);
	assert!(bad.body["error"].is_string());
}

/// If any statement fails partway (here: the second load insert), nothing is left behind —
/// no log, no loads, no new stores, no audit row.
#[sqlx::test(migrations = false)]
async fn a_failure_partway_rolls_back_everything(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let employee = ctx.employee(json!(42)).await;
	let audit_before = ctx.count("SELECT count(*) FROM audit_log").await;

	sqlx::raw_sql(
		"CREATE FUNCTION fail_on_1313() RETURNS trigger AS $$
         BEGIN
             IF NEW.pay = 13.13 THEN RAISE EXCEPTION 'simulated failure'; END IF;
             RETURN NEW;
         END $$ LANGUAGE plpgsql;
         CREATE TRIGGER fail_on_1313 BEFORE INSERT ON employee_loads
         FOR EACH ROW EXECUTE FUNCTION fail_on_1313();",
	)
	.execute(&ctx.pool)
	.await
	.unwrap();

	let loads = json!([
		{ "storeName": "New Store A", "loadDate": "2026-10-05", "pay": 100 },
		{ "storeName": "New Store B", "loadDate": "2026-10-06", "pay": 13.13 },
	]);
	let response = ctx
		.req(
			"POST",
			"/api/paycheck-logs",
			Some(&log_body(&employee, "2026-10-04", loads)),
		)
		.await;

	assert_eq!(response.status, 500);
	assert_eq!(
		response.body,
		json!({ "error": "Internal server error", "code": "INTERNAL_ERROR" })
	);
	assert_eq!(
		ctx.count("SELECT count(*) FROM employee_paycheck_logs")
			.await,
		0
	);
	assert_eq!(ctx.count("SELECT count(*) FROM employee_loads").await, 0);
	assert_eq!(ctx.count("SELECT count(*) FROM stores").await, 0);
	assert_eq!(
		ctx.count("SELECT count(*) FROM audit_log").await,
		audit_before
	);
}

/// Editing recomputes at the rate the log was created with, even after the
/// employee's rate changes.
#[sqlx::test(migrations = false)]
async fn edits_recompute_at_the_logs_own_rate(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let employee = ctx.employee(json!(42)).await;
	let created = ctx
		.req(
			"POST",
			"/api/paycheck-logs",
			Some(&log_body(&employee, "2026-10-04", one_load(json!(1000)))),
		)
		.await;
	let id = created.body["id"].as_str().unwrap();
	assert_eq!(num(&created.body["payRatePercent"]), 42.0);

	ctx.req("POST", &format!("/api/paycheck-logs/{id}/complete"), None)
		.await;
	ctx.req(
		"PUT",
		&format!("/api/employees/{employee}"),
		Some(&json!({ "payRatePercent": 40 })),
	)
	.await;

	let list = ctx.req("GET", "/api/paycheck-logs", None).await;
	assert_eq!(
		num(&list.body[0]["payRatePercent"]),
		42.0,
		"preview uses the log's rate"
	);

	let edit = json!({ "weekStart": "2026-10-04", "weekEnd": "2026-10-10", "payday": "2026-10-16", "loads": one_load(json!(2000)) });
	let response = ctx
		.req("PUT", &format!("/api/paycheck-logs/{id}"), Some(&edit))
		.await;
	assert_eq!(response.status, 200);
	assert_eq!(
		num(&response.body["employeeCut"]),
		840.0,
		"2000 × 42%, not × 40%"
	);
	assert_eq!(num(&response.body["payRatePercent"]), 42.0);
}

#[sqlx::test(migrations = false)]
async fn migration_backfills_existing_logs_with_the_employees_rate(pool: PgPool) {
	// Apply up to 0006, insert a pre-0007 log, then let the full migrate run 0007.
	let mut conn = pool.acquire().await.unwrap();
	sqlx::raw_sql(
        "CREATE TABLE schema_migrations (name text PRIMARY KEY, applied_at timestamptz NOT NULL DEFAULT now())",
    )
    .execute(&mut *conn)
    .await
    .unwrap();
	for (name, sql) in [
		("0001_init.sql", include_str!("../migrations/0001_init.sql")),
		(
			"0002_stores.sql",
			include_str!("../migrations/0002_stores.sql"),
		),
		(
			"0003_employee_loads_store_id.sql",
			include_str!("../migrations/0003_employee_loads_store_id.sql"),
		),
		(
			"0004_rename_weekly_log_to_paycheck_log.sql",
			include_str!("../migrations/0004_rename_weekly_log_to_paycheck_log.sql"),
		),
		(
			"0005_employee_pay_rate_percent.sql",
			include_str!("../migrations/0005_employee_pay_rate_percent.sql"),
		),
		(
			"0006_stores_name_lower_unique.sql",
			include_str!("../migrations/0006_stores_name_lower_unique.sql"),
		),
	] {
		sqlx::raw_sql(sql).execute(&mut *conn).await.unwrap();
		sqlx::query("INSERT INTO schema_migrations (name) VALUES ($1)")
			.bind(name)
			.execute(&mut *conn)
			.await
			.unwrap();
	}
	sqlx::raw_sql(
        "INSERT INTO employees (id, first_name, last_name, pay_rate_percent)
         VALUES ('00000000-0000-0000-0000-000000000001', 'A', 'B', 40);
         INSERT INTO employee_paycheck_logs (employee_id, week_start, week_end, payday, summary_total, employee_cut)
         VALUES ('00000000-0000-0000-0000-000000000001', '2026-01-04', '2026-01-10', '2026-01-16', 100, 40);",
    )
    .execute(&mut *conn)
    .await
    .unwrap();

	let applied = gr_logistics::cli::migrate::apply(&mut conn).await.unwrap();
	// Only the start matters: later migrations (unrelated to pay rates) run after these.
	assert_eq!(
		applied[..3],
		[
			"0007_paycheck_log_pay_rate.sql",
			"0008_paycheck_log_pay_rate_not_null.sql",
			"0009_paycheck_log_kind.sql"
		]
	);

	let rate: String =
		sqlx::query_scalar("SELECT pay_rate_percent::text FROM employee_paycheck_logs")
			.fetch_one(&mut *conn)
			.await
			.unwrap();
	assert_eq!(rate, "40.00");
}

fn lump_sum_body(employee_id: &str, week_start: &str, amount: Value) -> Value {
	json!({
		"employeeId": employee_id,
		"weekStart": week_start,
		"weekEnd": "2026-10-10",
		"payday": "2026-10-16",
		"lumpSum": amount,
	})
}

#[sqlx::test(migrations = false)]
async fn lump_sum_is_paid_in_full_with_no_loads(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let employee = ctx.employee(json!(42)).await;

	let response = ctx
		.req(
			"POST",
			"/api/paycheck-logs",
			Some(&lump_sum_body(&employee, "2026-10-04", json!(1234.56))),
		)
		.await;

	assert_eq!(response.status, 201, "{:?}", response.body);
	let log = &response.body;
	assert_eq!(log["kind"], "lump_sum");
	assert_eq!(log["status"], "draft");
	// No cut: total and pay are both the amount, cents kept, recorded at 100%.
	assert_eq!(log["summaryTotal"].to_string(), "1234.56");
	assert_eq!(log["employeeCut"].to_string(), "1234.56");
	assert_eq!(num(&log["payRatePercent"]), 100.0);
	assert_eq!(ctx.count("SELECT count(*) FROM employee_loads").await, 0);
	assert_eq!(
		ctx.count(
			"SELECT count(*) FROM audit_log WHERE entity = 'employee_paycheck_log' AND action = 'create'"
		)
		.await,
		1
	);

	let list = ctx.req("GET", "/api/paycheck-logs", None).await;
	assert_eq!(list.body[0]["kind"], "lump_sum");

	let id = log["id"].as_str().unwrap();
	let loads = ctx
		.req("GET", &format!("/api/paycheck-logs/{id}/loads"), None)
		.await;
	assert_eq!(loads.body, json!([]));
}

#[sqlx::test(migrations = false)]
async fn loads_logs_report_their_kind(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let employee = ctx.employee(json!(42)).await;

	let response = ctx
		.req(
			"POST",
			"/api/paycheck-logs",
			Some(&log_body(&employee, "2026-10-04", one_load(json!(100)))),
		)
		.await;
	assert_eq!(response.status, 201);
	assert_eq!(response.body["kind"], "loads");
}

#[sqlx::test(migrations = false)]
async fn invalid_lump_sums_are_400_and_write_nothing(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let employee = ctx.employee(json!(42)).await;

	let mut both = lump_sum_body(&employee, "2026-10-04", json!(100));
	both["loads"] = one_load(json!(100));

	let cases = [
		both,
		lump_sum_body(&employee, "2026-10-04", json!(0)),
		lump_sum_body(&employee, "2026-10-04", json!(-5)),
		lump_sum_body(&employee, "2026-10-04", json!(1.005)),
		lump_sum_body(&employee, "2026-10-04", json!("100")),
		lump_sum_body(&employee, "2026-10-04", json!(100000000)),
	];

	for body in &cases {
		let response = ctx.req("POST", "/api/paycheck-logs", Some(body)).await;
		assert_eq!(response.status, 400, "{body}: {:?}", response.body);
		assert!(response.body["error"].is_string());
	}

	assert_eq!(
		ctx.count("SELECT count(*) FROM employee_paycheck_logs")
			.await,
		0
	);
}

#[sqlx::test(migrations = false)]
async fn lump_sum_edits_keep_their_kind(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let employee = ctx.employee(json!(42)).await;

	let lump = ctx
		.req(
			"POST",
			"/api/paycheck-logs",
			Some(&lump_sum_body(&employee, "2026-10-04", json!(800))),
		)
		.await;
	let lump_id = lump.body["id"].as_str().unwrap().to_owned();
	let other = ctx.employee(json!(42)).await;
	let loads = ctx
		.req(
			"POST",
			"/api/paycheck-logs",
			Some(&log_body(&other, "2026-10-04", one_load(json!(100)))),
		)
		.await;
	let loads_id = loads.body["id"].as_str().unwrap().to_owned();

	let edit = json!({
		"weekStart": "2026-10-04", "weekEnd": "2026-10-10", "payday": "2026-10-17",
		"lumpSum": 950.5,
	});
	let response = ctx
		.req("PUT", &format!("/api/paycheck-logs/{lump_id}"), Some(&edit))
		.await;
	assert_eq!(response.status, 200, "{:?}", response.body);
	assert_eq!(num(&response.body["summaryTotal"]), 950.5);
	assert_eq!(num(&response.body["employeeCut"]), 950.5);
	assert_eq!(response.body["payday"], "2026-10-17");

	// A lump sum can't become a loads log, nor the other way round.
	let to_loads = json!({
		"weekStart": "2026-10-04", "weekEnd": "2026-10-10", "payday": "2026-10-17",
		"loads": one_load(json!(100)),
	});
	let response = ctx
		.req(
			"PUT",
			&format!("/api/paycheck-logs/{lump_id}"),
			Some(&to_loads),
		)
		.await;
	assert_eq!(response.status, 400, "{:?}", response.body);

	let to_lump = json!({
		"weekStart": "2026-10-04", "weekEnd": "2026-10-10", "payday": "2026-10-16",
		"lumpSum": 100,
	});
	let response = ctx
		.req(
			"PUT",
			&format!("/api/paycheck-logs/{loads_id}"),
			Some(&to_lump),
		)
		.await;
	assert_eq!(response.status, 400, "{:?}", response.body);
	assert_eq!(ctx.count("SELECT count(*) FROM employee_loads").await, 1);
}

#[sqlx::test(migrations = false)]
async fn create_can_complete_in_one_step(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let employee = ctx.employee(json!(42)).await;

	let mut body = log_body(&employee, "2026-10-04", one_load(json!(1000)));
	body["complete"] = json!(true);
	let response = ctx.req("POST", "/api/paycheck-logs", Some(&body)).await;
	assert_eq!(response.status, 201, "{:?}", response.body);
	assert_eq!(response.body["status"], "completed");
	assert_eq!(num(&response.body["employeeCut"]), 420.0);

	let mut lump = lump_sum_body(&employee, "2026-10-11", json!(500));
	lump["weekEnd"] = json!("2026-10-17");
	lump["complete"] = json!(true);
	let response = ctx.req("POST", "/api/paycheck-logs", Some(&lump)).await;
	assert_eq!(response.status, 201, "{:?}", response.body);
	assert_eq!(response.body["status"], "completed");

	let diff: (Value,) = sqlx::query_as(
        "SELECT diff FROM audit_log WHERE entity = 'employee_paycheck_log' AND action = 'create' ORDER BY created_at LIMIT 1",
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();
	assert_eq!(diff.0, json!({ "status": "completed" }));

	// Omitted still means draft.
	let other = ctx.employee(json!(42)).await;
	let draft = ctx
		.req(
			"POST",
			"/api/paycheck-logs",
			Some(&log_body(&other, "2026-10-04", one_load(json!(10)))),
		)
		.await;
	assert_eq!(draft.status, 201, "{:?}", draft.body);
	assert_eq!(draft.body["status"], "draft");
}

#[sqlx::test(migrations = false)]
async fn edit_can_complete_a_draft_in_one_step(pool: PgPool) {
	let ctx = Ctx::new(pool).await;
	let employee = ctx.employee(json!(42)).await;

	let draft = ctx
		.req(
			"POST",
			"/api/paycheck-logs",
			Some(&log_body(&employee, "2026-10-04", one_load(json!(100)))),
		)
		.await;
	let id = draft.body["id"].as_str().unwrap().to_owned();

	// Saving without `complete` keeps it a draft.
	let mut edit = json!({
		"weekStart": "2026-10-04", "weekEnd": "2026-10-10", "payday": "2026-10-16",
		"loads": one_load(json!(1000)),
	});
	let response = ctx
		.req("PUT", &format!("/api/paycheck-logs/{id}"), Some(&edit))
		.await;
	assert_eq!(response.status, 200, "{:?}", response.body);
	assert_eq!(response.body["status"], "draft");

	// With `complete`, the edit lands and the log is completed together.
	edit["loads"] = one_load(json!(2000));
	edit["complete"] = json!(true);
	let response = ctx
		.req("PUT", &format!("/api/paycheck-logs/{id}"), Some(&edit))
		.await;
	assert_eq!(response.status, 200, "{:?}", response.body);
	assert_eq!(response.body["status"], "completed");
	assert_eq!(num(&response.body["employeeCut"]), 840.0);

	// A later plain save never reopens it.
	edit.as_object_mut().unwrap().remove("complete");
	let response = ctx
		.req("PUT", &format!("/api/paycheck-logs/{id}"), Some(&edit))
		.await;
	assert_eq!(response.body["status"], "completed");
}

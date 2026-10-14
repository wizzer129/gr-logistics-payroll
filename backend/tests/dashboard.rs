//! Runs against real Postgres (`#[sqlx::test]`, DATABASE_URL from `varlock run` / .env.schema).

mod common;

use axum::Router;
use common::{app_with_db, insert_user, login, send};
use serde_json::{Value, json};
use sqlx::PgPool;

fn num(value: &Value) -> f64 {
    value
        .as_f64()
        .unwrap_or_else(|| panic!("not a JSON number: {value}"))
}

struct Seed {
    app: Router,
    cookie: String,
    ana: String,
    bo: String,
}

/// Ana (42%): Jan 2026 completed ($1000 → 420), Feb 2026 completed ($500 + $50 → 231),
///            Mar 2026 draft ($300), Dec 28 2025 completed (previous year).
/// Bo (40%):  Jan 2026 completed ($2000 → 800).
async fn seed(pool: PgPool) -> Seed {
    let app = app_with_db(pool.clone()).await;
    insert_user(&pool, "admin@example.com", "admin").await;
    let cookie = login(&app, "admin@example.com").await;

    let employee = |first: &'static str, last: &'static str, rate: i32| {
        let (app, cookie) = (app.clone(), cookie.clone());
        async move {
            let body =
                json!({ "firstName": first, "lastName": last, "payRatePercent": rate }).to_string();
            let response = send(&app, "POST", "/api/employees", Some(&body), Some(&cookie)).await;
            response.body["id"].as_str().unwrap().to_owned()
        }
    };
    let ana = employee("Ana", "Diaz", 42).await;
    let bo = employee("Bo", "Lee", 40).await;

    for (employee, week_start, loads, complete) in [
        (
            &ana,
            "2026-01-04",
            json!([{ "storeName": "A", "loadDate": "2026-01-05", "pay": 1000 }]),
            true,
        ),
        (
            &ana,
            "2026-02-01",
            json!([
                { "storeName": "A", "loadDate": "2026-02-02", "pay": 500 },
                { "storeName": "B", "loadDate": "2026-02-03", "pay": 0, "extraPay": 50 },
            ]),
            true,
        ),
        (
            &ana,
            "2026-03-01",
            json!([{ "storeName": "A", "loadDate": "2026-03-02", "pay": 300 }]),
            false,
        ),
        (
            &ana,
            "2025-12-28",
            json!([{ "storeName": "A", "loadDate": "2025-12-29", "pay": 100 }]),
            true,
        ),
        (
            &bo,
            "2026-01-04",
            json!([{ "storeName": "C", "loadDate": "2026-01-06", "pay": 2000 }]),
            true,
        ),
    ] {
        let body = json!({
            "employeeId": employee, "weekStart": week_start, "weekEnd": week_start,
            "payday": week_start, "loads": loads,
        })
        .to_string();
        let created = send(
            &app,
            "POST",
            "/api/paycheck-logs",
            Some(&body),
            Some(&cookie),
        )
        .await;
        assert_eq!(created.status, 201, "{:?}", created.body);
        if complete {
            let id = created.body["id"].as_str().unwrap();
            let uri = format!("/api/paycheck-logs/{id}/complete");
            send(&app, "POST", &uri, None, Some(&cookie)).await;
        }
    }

    Seed {
        app,
        cookie,
        ana,
        bo,
    }
}

#[sqlx::test(migrations = false)]
async fn summary_counts_completed_logs_in_the_year(pool: PgPool) {
    let s = seed(pool).await;
    let response = send(
        &s.app,
        "GET",
        "/api/dashboard/summary?year=2026",
        None,
        Some(&s.cookie),
    )
    .await;

    assert_eq!(response.status, 200, "{:?}", response.body);
    let body = &response.body;
    assert_eq!(body["year"], 2026);
    assert_eq!(num(&body["ytdTotal"]), 420.0 + 231.0 + 800.0);
    assert_eq!(num(&body["ytdGross"]), 1000.0 + 550.0 + 2000.0);
    assert_eq!(body["paycheckCount"], 3, "draft and 2025 logs excluded");
    assert_eq!(body["totalLoads"], 4);

    let by_employee: Vec<_> = body["byEmployee"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["firstName"].as_str().unwrap().to_owned(),
                num(&row["total"]),
            )
        })
        .collect();
    assert_eq!(
        by_employee,
        [("Bo".to_owned(), 800.0), ("Ana".to_owned(), 651.0)]
    );
    assert_eq!(body["byEmployee"][0]["employeeId"], s.bo);

    let by_month: Vec<_> = body["byMonth"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            format!(
                "{} {} {}",
                row["period"].as_str().unwrap(),
                row["firstName"].as_str().unwrap(),
                num(&row["total"])
            )
        })
        .collect();
    assert_eq!(
        by_month,
        ["2026-01 Ana 420", "2026-01 Bo 800", "2026-02 Ana 231"]
    );
}

#[sqlx::test(migrations = false)]
async fn summary_year_param(pool: PgPool) {
    let s = seed(pool).await;

    let previous = send(
        &s.app,
        "GET",
        "/api/dashboard/summary?year=2025",
        None,
        Some(&s.cookie),
    )
    .await;
    assert_eq!(previous.body["paycheckCount"], 1);
    assert_eq!(num(&previous.body["ytdTotal"]), 42.0);

    let empty = send(
        &s.app,
        "GET",
        "/api/dashboard/summary?year=2030",
        None,
        Some(&s.cookie),
    )
    .await;
    assert_eq!(empty.status, 200);
    assert_eq!(num(&empty.body["ytdTotal"]), 0.0);
    assert_eq!(empty.body["byEmployee"], json!([]));

    let default = send(
        &s.app,
        "GET",
        "/api/dashboard/summary",
        None,
        Some(&s.cookie),
    )
    .await;
    assert_eq!(
        default.body["year"],
        chrono::Datelike::year(&chrono::Utc::now())
    );

    for bad in ["1999", "2101", "abc", "2026.5"] {
        let uri = format!("/api/dashboard/summary?year={bad}");
        let response = send(&s.app, "GET", &uri, None, Some(&s.cookie)).await;
        assert_eq!(response.status, 400, "year={bad}");
        assert!(response.body["error"].is_string());
    }
}

#[sqlx::test(migrations = false)]
async fn employee_summary_lists_drafts_but_totals_completed_only(pool: PgPool) {
    let s = seed(pool).await;
    let uri = format!("/api/dashboard/employee/{}/summary?year=2026", s.ana);
    let response = send(&s.app, "GET", &uri, None, Some(&s.cookie)).await;

    assert_eq!(response.status, 200);
    let body = &response.body;
    assert_eq!(
        body["employee"],
        json!({ "id": s.ana, "firstName": "Ana", "lastName": "Diaz" })
    );
    assert_eq!(num(&body["ytdTotal"]), 651.0);

    let logs: Vec<_> = body["logs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|log| {
            format!(
                "{} {}",
                log["weekStart"].as_str().unwrap(),
                log["status"].as_str().unwrap()
            )
        })
        .collect();
    assert_eq!(
        logs,
        [
            "2026-01-04 completed",
            "2026-02-01 completed",
            "2026-03-01 draft"
        ]
    );
    assert_eq!(body["logs"][0]["payday"], "2026-01-04", "date-only");

    let missing = format!("/api/dashboard/employee/{}/summary", uuid::Uuid::new_v4());
    let response = send(&s.app, "GET", &missing, None, Some(&s.cookie)).await;
    assert_eq!(response.status, 404);
}

mod common;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use tower::ServiceExt;

async fn status(method: Method, uri: &str) -> StatusCode {
    common::app_without_db()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

/// Public (no 401), and honest: with no reachable database it reports 503.
#[tokio::test]
async fn health_is_public_and_checks_the_database() {
    assert_eq!(
        status(Method::GET, "/api/health").await,
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[sqlx::test(migrations = false)]
async fn health_is_ok_with_a_database(pool: sqlx::PgPool) {
    let app = common::app_with_db(pool).await;
    let response = common::send(&app, "GET", "/api/health", None, None).await;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body, serde_json::json!({ "status": "ok" }));
}

#[tokio::test]
async fn unknown_route_is_404() {
    assert_eq!(
        status(Method::GET, "/api/nope").await,
        StatusCode::NOT_FOUND
    );
}

/// Every data route requires a session — including the three GETs the Bun
/// backend left open.
#[tokio::test]
async fn protected_routes_reject_without_session() {
    let id = "00000000-0000-0000-0000-000000000000";
    let routes = [
        (Method::GET, "/api/auth/me".to_owned()),
        (Method::GET, "/api/employees".to_owned()),
        (Method::POST, "/api/employees".to_owned()),
        (Method::PUT, format!("/api/employees/{id}")),
        (Method::DELETE, format!("/api/employees/{id}")),
        (Method::GET, "/api/paycheck-logs".to_owned()),
        (Method::POST, "/api/paycheck-logs".to_owned()),
        (Method::PUT, format!("/api/paycheck-logs/{id}")),
        (Method::DELETE, format!("/api/paycheck-logs/{id}")),
        (Method::POST, format!("/api/paycheck-logs/{id}/complete")),
        (Method::GET, format!("/api/paycheck-logs/{id}/loads")),
        (Method::GET, "/api/dashboard/summary".to_owned()),
        (Method::GET, format!("/api/dashboard/employee/{id}/summary")),
        (Method::POST, "/api/loads/extract".to_owned()),
    ];

    for (method, uri) in routes {
        assert_eq!(
            status(method.clone(), &uri).await,
            StatusCode::UNAUTHORIZED,
            "{method} {uri} should require a session"
        );
    }
}

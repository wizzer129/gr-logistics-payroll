// Each integration-test crate compiles this module separately and uses only some helpers.
#![allow(dead_code)]

use std::sync::Arc;

use axum::Router;
use gr_logistics::{
    build_app,
    config::{Config, Environment, LogFormat},
    state::AppState,
};
use sqlx::postgres::PgPoolOptions;

pub fn test_config() -> Config {
    Config {
        environment: Environment::Development,
        port: 0,
        database_url: "postgres://unused@localhost/unused".into(),
        session_ttl_hours: 12,
        cors_origin: "http://localhost:5173".parse().unwrap(),
        log_level: "error".into(),
        log_format: LogFormat::Pretty,
    }
}

/// App without a live database — the lazy pool never connects unless a query runs. For
/// tests that touch the DB, use `#[sqlx::test]` and build `AppState` from its pool instead.
pub fn app_without_db() -> Router {
    let config = test_config();
    let pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(1))
        .connect_lazy(&config.database_url)
        .unwrap();

    build_app(AppState {
        pool,
        config: Arc::new(config),
    })
}

/// App backed by a `#[sqlx::test]` database, with our migrations applied.
pub async fn app_with_db(pool: sqlx::PgPool) -> Router {
    app_with_db_and_config(pool, test_config()).await
}

pub async fn app_with_db_and_config(pool: sqlx::PgPool, config: Config) -> Router {
    let mut conn = pool.acquire().await.unwrap();
    gr_logistics::cli::migrate::apply(&mut conn).await.unwrap();
    drop(conn);

    build_app(AppState {
        pool,
        config: Arc::new(config),
    })
}

/// `Bun.password.hash('bun-fixture-password', { algorithm: 'argon2id' })`.
pub const BUN_FIXTURE_HASH: &str = "$argon2id$v=19$m=65536,t=2,p=1$qGwulwFjunboRqjguU6ZO91eDZin/SISpO23hKsKNP4$O8/J8ZsJ7En4WOCEy6+DnUg4dcoysbY/n2UGT2ZSVUA";
pub const BUN_FIXTURE_PASSWORD: &str = "bun-fixture-password";

/// Inserts a user whose password hash was written by the Bun backend.
pub async fn insert_user(pool: &sqlx::PgPool, email: &str, role: &str) -> uuid::Uuid {
    sqlx::query_scalar(
        "INSERT INTO users (email, password_hash, role) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(email)
    .bind(BUN_FIXTURE_HASH)
    .bind(role)
    .fetch_one(pool)
    .await
    .unwrap()
}

pub struct TestResponse {
    pub status: axum::http::StatusCode,
    pub headers: axum::http::HeaderMap,
    pub body: serde_json::Value,
}

pub async fn send(
    app: &Router,
    method: &str,
    uri: &str,
    body: Option<&str>,
    cookie: Option<&str>,
) -> TestResponse {
    use tower::ServiceExt;

    let mut request = axum::http::Request::builder().method(method).uri(uri);
    if body.is_some() {
        request = request.header("content-type", "application/json");
    }
    if let Some(cookie) = cookie {
        request = request.header("cookie", cookie);
    }
    let request = request
        .body(axum::body::Body::from(body.unwrap_or_default().to_owned()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);

    TestResponse {
        status,
        headers,
        body,
    }
}

/// Logs in and returns the `gr_session=<id>` pair to send back as a Cookie header.
pub async fn login(app: &Router, email: &str) -> String {
    let body = format!(r#"{{"email":"{email}","password":"{BUN_FIXTURE_PASSWORD}"}}"#);
    let response = send(app, "POST", "/api/auth/login", Some(&body), None).await;
    assert_eq!(response.status, 200, "login failed: {:?}", response.body);

    let set_cookie = response.headers["set-cookie"].to_str().unwrap();
    set_cookie.split(';').next().unwrap().to_owned()
}

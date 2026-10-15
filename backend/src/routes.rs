use std::time::Duration;

use axum::{
    Router,
    extract::Request,
    http::{Method, StatusCode, header},
    middleware,
    response::Response,
    routing::{get, post, put},
};
use tower_http::{cors::CorsLayer, timeout::TimeoutLayer, trace::TraceLayer};
use tracing::Span;

use crate::{
    auth,
    error::AppError,
    handlers::{auth as auth_handlers, dashboard, employees, health, paycheck_logs},
    state::AppState,
};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub fn build_app(state: AppState) -> Router {
    let timeout =
        |duration| TimeoutLayer::with_status_code(StatusCode::SERVICE_UNAVAILABLE, duration);

    let public = Router::new()
        .route("/api/auth/login", post(auth_handlers::login))
        .route("/api/auth/logout", post(auth_handlers::logout))
        .route("/api/health", get(health::health))
        .layer(timeout(REQUEST_TIMEOUT));

    let data = Router::new()
        .route("/api/auth/me", get(auth_handlers::me))
        .route(
            "/api/employees",
            get(employees::list).post(employees::create),
        )
        .route(
            "/api/employees/{id}",
            put(employees::update).delete(employees::delete),
        )
        .route(
            "/api/paycheck-logs",
            get(paycheck_logs::list).post(paycheck_logs::create),
        )
        .route(
            "/api/paycheck-logs/{id}",
            put(paycheck_logs::update).delete(paycheck_logs::delete),
        )
        .route(
            "/api/paycheck-logs/{id}/complete",
            post(paycheck_logs::complete),
        )
        .route("/api/paycheck-logs/{id}/loads", get(paycheck_logs::loads))
        .route("/api/dashboard/summary", get(dashboard::summary))
        .route(
            "/api/dashboard/employee/{id}/summary",
            get(dashboard::employee_summary),
        )
        .layer(timeout(REQUEST_TIMEOUT));

    // Everything here requires a session. Handlers also take `AuthUser` and check the role
    // explicitly, so a route accidentally added outside this router still can't skip auth.
    let protected = data.route_layer(middleware::from_fn_with_state(
        state.clone(),
        auth::middleware::require_session,
    ));

    Router::new()
        .merge(public)
        .merge(protected)
        .fallback(|| async { AppError::NotFound("Not found".into()) })
        // One INFO line per request — `request{method=GET path=/api/employees}: request
        // handled status=200 latency_ms=3`. Errors
        // are already logged by `AppError`, so tower-http's start/end/failure events are off.
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request| {
                    // Path only: query strings stay out of the logs.
                    tracing::info_span!(
                        "request",
                        method = %request.method(),
                        path = %request.uri().path()
                    )
                })
                .on_request(())
                .on_response(|response: &Response, latency: Duration, _span: &Span| {
                    tracing::info!(
                        status = response.status().as_u16(),
                        latency_ms = u64::try_from(latency.as_millis()).unwrap_or(u64::MAX),
                        "request handled"
                    );
                })
                .on_body_chunk(())
                .on_eos(())
                .on_failure(()),
        )
        .layer(cors_layer(&state))
        .with_state(state)
}

fn cors_layer(state: &AppState) -> CorsLayer {
    CorsLayer::new()
        .allow_origin(state.config.cors_origin.clone())
        .allow_credentials(true)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE])
}

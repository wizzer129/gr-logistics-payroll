use std::time::Duration;

use axum::{
	Router,
	extract::Request,
	http::{Method, header},
	middleware::{self, Next},
	response::{IntoResponse, Response},
	routing::{get, post, put},
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::Span;

use crate::{
	auth,
	error::{AppError, ErrorCode},
	handlers::{auth as auth_handlers, dashboard, email, employees, health, paycheck_logs},
	state::AppState,
};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub fn build_app(state: AppState) -> Router {
	let public = Router::new()
		.route("/api/auth/login", post(auth_handlers::login))
		.route("/api/auth/logout", post(auth_handlers::logout))
		.route(
			"/api/auth/password-reset",
			post(auth_handlers::request_password_reset),
		)
		.route(
			"/api/auth/password-reset/confirm",
			post(auth_handlers::confirm_password_reset),
		)
		.route("/api/health", get(health::health))
		.layer(middleware::from_fn(timeout));

	let data = Router::new()
		.route("/api/auth/me", get(auth_handlers::me))
		.route("/api/auth/me/locale", put(auth_handlers::update_locale))
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
		.route("/api/paycheck-logs/{id}/email", post(email::send_paystub))
		.route("/api/dashboard/summary", get(dashboard::summary))
		.route(
			"/api/email/recipient-options",
			get(email::recipient_options),
		)
		.route(
			"/api/email/monthly-report/recipients",
			get(email::monthly_report_recipients).put(email::set_monthly_report_recipients),
		)
		.route(
			"/api/email/monthly-report",
			post(email::send_monthly_report),
		)
		.route("/api/email/outbox", get(email::outbox))
		.route("/api/email/outbox/{id}/retry", post(email::retry_email))
		.route(
			"/api/dashboard/employee/{id}/summary",
			get(dashboard::employee_summary),
		)
		.layer(middleware::from_fn(timeout));

	// Everything here requires a session. Handlers also take `AuthUser` and check the role
	// explicitly, so a route accidentally added outside this router still can't skip auth.
	let protected = data.route_layer(middleware::from_fn_with_state(
		state.clone(),
		auth::middleware::require_session,
	));

	Router::new()
		.merge(public)
		.merge(protected)
		.fallback(|| async { AppError::NotFound(ErrorCode::NotFound, "Not found".into()) })
		// axum's own 405 has an empty body; this gives it the `{ error, code }` shape.
		.method_not_allowed_fallback(|| async {
			AppError::MethodNotAllowed(ErrorCode::MethodNotAllowed, "Method not allowed".into())
		})
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

/// Cuts a request off after `REQUEST_TIMEOUT` with a coded 503 (tower-http's `TimeoutLayer`
/// answers with an empty body, which the frontend can't translate).
async fn timeout(request: Request, next: Next) -> Response {
	with_timeout(REQUEST_TIMEOUT, request, next).await
}

async fn with_timeout(limit: Duration, request: Request, next: Next) -> Response {
	match tokio::time::timeout(limit, next.run(request)).await {
		Ok(response) => response,
		Err(_) => AppError::ServiceUnavailable(
			ErrorCode::RequestTimeout,
			format!("Request timed out after {}s", limit.as_secs()),
		)
		.into_response(),
	}
}

fn cors_layer(state: &AppState) -> CorsLayer {
	CorsLayer::new()
		.allow_origin(state.config.cors_origin.clone())
		.allow_credentials(true)
		.allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
		.allow_headers([header::CONTENT_TYPE])
}

#[cfg(test)]
mod tests {
	use axum::body::{Body, to_bytes};
	use tower::ServiceExt;

	use super::*;

	#[tokio::test]
	async fn a_slow_request_times_out_with_a_coded_503() {
		let app = Router::new()
			.route(
				"/slow",
				get(|| async { tokio::time::sleep(Duration::from_secs(5)).await }),
			)
			.layer(middleware::from_fn(|request, next| {
				with_timeout(Duration::from_millis(10), request, next)
			}));

		let request = Request::builder().uri("/slow").body(Body::empty()).unwrap();
		let response = app.oneshot(request).await.unwrap();

		assert_eq!(response.status(), 503);
		let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
		let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
		assert_eq!(body["code"], "REQUEST_TIMEOUT");
	}
}

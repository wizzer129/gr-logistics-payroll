use axum::{Json, extract::State, http::StatusCode};
use serde_json::{Value, json};

use crate::state::AppState;

/// Public. 503 when the database is unreachable, so the container healthcheck (and any
/// orchestrator) restarts an instance that lost its pool.
pub async fn health(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
	match sqlx::query("SELECT 1").execute(&state.pool).await {
		Ok(_) => (StatusCode::OK, Json(json!({ "status": "ok" }))),
		Err(error) => {
			tracing::error!(%error, "health check: database unreachable");
			(
				StatusCode::SERVICE_UNAVAILABLE,
				Json(json!({ "status": "error", "error": "Database unavailable" })),
			)
		}
	}
}

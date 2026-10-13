use std::sync::Arc;

use sqlx::PgPool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
    /// Shared client for the Anthropic API (connection pooling across requests).
    pub http: reqwest::Client,
}

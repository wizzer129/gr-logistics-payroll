use axum::http::HeaderValue;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Environment {
    Development,
    Production,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogFormat {
    Pretty,
    Json,
}

/// No `Debug` derive — this holds secrets and must never end up in a log line.
#[derive(Clone)]
pub struct Config {
    pub environment: Environment,
    pub port: u16,
    pub database_url: String,
    pub session_ttl_hours: i64,
    pub cors_origin: HeaderValue,
    pub log_level: String,
    pub log_format: LogFormat,
}

/// Must be set (non-empty) or startup fails. Keep `.env.schema` in sync (tests/env_schema.rs).
pub const REQUIRED_KEYS: &[&str] = &[
    "NODE_ENV",
    "PORT",
    "DATABASE_URL",
    "SESSION_TTL_HOURS",
    "CORS_ORIGIN",
    "LOG_LEVEL",
];

/// Read when present, with a default otherwise.
pub const OPTIONAL_KEYS: &[&str] = &["LOG_FORMAT"];

impl Config {
    /// Fails fast listing every missing key at once, like the TS `assertEnv()`.
    pub fn from_env() -> anyhow::Result<Self> {
        let missing: Vec<_> = REQUIRED_KEYS
            .iter()
            .filter(|key| std::env::var(key).map_or(true, |v| v.is_empty()))
            .copied()
            .collect();

        if !missing.is_empty() {
            anyhow::bail!(
                "Missing required environment variables: {}",
                missing.join(", ")
            );
        }

        let environment = match var("NODE_ENV").as_str() {
            "production" => Environment::Production,
            _ => Environment::Development,
        };

        let log_format = match std::env::var("LOG_FORMAT").ok().as_deref() {
            Some("json") => LogFormat::Json,
            Some("pretty") => LogFormat::Pretty,
            _ if environment == Environment::Development => LogFormat::Pretty,
            _ => LogFormat::Json,
        };

        Ok(Self {
            environment,
            port: var("PORT").parse()?,
            database_url: var("DATABASE_URL"),
            session_ttl_hours: var("SESSION_TTL_HOURS").parse()?,
            cors_origin: var("CORS_ORIGIN").parse()?,
            log_level: var("LOG_LEVEL"),
            log_format,
        })
    }

    pub fn is_production(&self) -> bool {
        self.environment == Environment::Production
    }
}

/// Only called after the REQUIRED_KEYS check, so a missing key is already impossible.
fn var(key: &str) -> String {
    std::env::var(key).unwrap_or_default()
}

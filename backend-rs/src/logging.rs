use tracing_subscriber::EnvFilter;

use crate::config::{Config, LogFormat};

/// JSON for log collectors in production, human-readable lines in development.
///
/// Scopes: `tracing` tags every event with its module path (`gr_logistics::services::stores`),
/// which replaces `createLogger('scope')` — don't pass a scope by hand.
///
/// TODO(phase 1): custom `FormatEvent` for `Pretty` that colors each target via FNV-1a into
/// the same 32-color palette as `backend/src/utils/logger.ts`.
pub fn init(config: &Config) {
    // LOG_LEVEL applies to our own code and request traces; dependencies (sqlx logs every
    // query at DEBUG) stay at WARN, matching the Bun logger, which only ever logged app code.
    let level = config.log_level.to_lowercase();
    let filter = EnvFilter::try_new(format!("warn,gr_logistics={level},tower_http={level}"))
        .unwrap_or_else(|_| EnvFilter::new("warn,gr_logistics=info,tower_http=info"));

    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true);

    match config.log_format {
        LogFormat::Json => builder.json().init(),
        LogFormat::Pretty => builder.init(),
    }
}

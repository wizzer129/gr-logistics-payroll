//! Hourly housekeeping in `serve`: expired sessions and password reset tokens are otherwise
//! only removed on logout or a reset, so both tables would grow forever.

use std::time::Duration;

use sqlx::PgPool;
use tokio::sync::watch;

use crate::{auth::session, services::password_resets};

const EVERY: Duration = Duration::from_secs(60 * 60);

/// One pass; returns `(sessions, reset tokens)` deleted.
pub async fn tick(pool: &PgPool) -> Result<(u64, u64), sqlx::Error> {
	let sessions = session::delete_expired(pool).await?;
	let tokens = password_resets::delete_expired(pool).await?;
	if sessions + tokens > 0 {
		tracing::info!(sessions, tokens, "deleted expired rows");
	}
	Ok((sessions, tokens))
}

/// Runs until `shutdown` flips to `true`.
pub async fn run(pool: PgPool, mut shutdown: watch::Receiver<bool>) {
	loop {
		if let Err(error) = tick(&pool).await {
			tracing::error!(%error, "cleanup failed");
		}
		tokio::select! {
			_ = shutdown.changed() => break,
			() = tokio::time::sleep(EVERY) => {},
		}
		if *shutdown.borrow() {
			break;
		}
	}
}

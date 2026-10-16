pub mod auth;
pub mod cli;
pub mod config;
pub mod domain;
pub mod error;
pub mod extract;
pub mod handlers;
pub mod logging;
pub mod models;
pub mod routes;
pub mod services;
pub mod state;

use std::{sync::Arc, time::Duration};

use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;

pub use routes::build_app;

use crate::{config::Config, state::AppState};

pub async fn serve(config: Config) -> anyhow::Result<()> {
	let pool = PgPoolOptions::new()
		.max_connections(10)
		.acquire_timeout(Duration::from_secs(5))
		.connect(&config.database_url)
		.await?;

	let port = config.port;
	let state = AppState {
		pool,
		config: Arc::new(config),
	};

	let listener = TcpListener::bind(("0.0.0.0", port)).await?;
	tracing::info!(port, "backend listening");

	axum::serve(listener, build_app(state))
		.with_graceful_shutdown(shutdown_signal())
		.await?;

	Ok(())
}

/// Resolves on Ctrl+C or SIGTERM so in-flight requests (e.g. a paycheck write) finish
/// before the process exits on deploy.
async fn shutdown_signal() {
	let ctrl_c = async {
		tokio::signal::ctrl_c().await.ok();
	};

	#[cfg(unix)]
	let terminate = async {
		if let Ok(mut signal) =
			tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
		{
			signal.recv().await;
		}
	};

	#[cfg(not(unix))]
	let terminate = std::future::pending::<()>();

	tokio::select! {
		() = ctrl_c => {},
		() = terminate => {},
	}

	tracing::info!("shutdown signal received");
}

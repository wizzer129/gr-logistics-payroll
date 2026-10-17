pub mod auth;
pub mod cli;
pub mod config;
pub mod domain;
pub mod email;
pub mod error;
pub mod extract;
pub mod handlers;
pub mod locale;
pub mod logging;
pub mod models;
pub mod pdf;
pub mod routes;
pub mod services;
pub mod state;

use std::{sync::Arc, time::Duration};

use sqlx::postgres::PgPoolOptions;
use tokio::{net::TcpListener, sync::watch};

pub use routes::build_app;

use crate::{
	config::Config,
	email::{outbox::Outbox, render::Renderer, sender},
	state::AppState,
};

/// reqwest and sqlx both use rustls with the `ring` backend; reqwest needs it installed as
/// the process default (`rustls-no-provider`, to avoid compiling a second backend). Safe to
/// call more than once.
pub fn install_tls_provider() {
	let _ = rustls::crypto::ring::default_provider().install_default();
}

pub async fn serve(config: Config) -> anyhow::Result<()> {
	let pool = PgPoolOptions::new()
		.max_connections(10)
		.acquire_timeout(Duration::from_secs(5))
		.connect(&config.database_url)
		.await?;

	let port = config.port;
	let outbox = Outbox::new(
		pool.clone(),
		sender::from_config(&config.email),
		Arc::new(Renderer::new()),
	);
	let state = AppState {
		pool,
		config: Arc::new(config),
	};

	// One signal stops both: the HTTP server drains in-flight requests, and the outbox
	// worker finishes the send it's on.
	let (shutdown_tx, shutdown_rx) = watch::channel(false);
	let worker = tokio::spawn(outbox.run(shutdown_rx));

	let listener = TcpListener::bind(("0.0.0.0", port)).await?;
	tracing::info!(port, "backend listening");

	axum::serve(listener, build_app(state))
		.with_graceful_shutdown(async move {
			shutdown_signal().await;
			let _ = shutdown_tx.send(true);
		})
		.await?;

	worker.await?;
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

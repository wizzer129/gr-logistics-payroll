//! `gr-logistics healthcheck` — for the container HEALTHCHECK. The distroless image has no
//! curl or shell, so the binary checks itself: GET /api/health on the local port, exit 0 or 1.
//! Needs only PORT (not the full config).

use std::time::Duration;

pub async fn run() -> anyhow::Result<()> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let url = format!("http://127.0.0.1:{port}/api/health");

    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()?
        .get(&url)
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("{url} returned {}", response.status());
    }
    Ok(())
}

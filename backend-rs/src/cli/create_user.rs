//! Port of backend/scripts/create-user.ts — upserts on email, so it also resets a password.

use anyhow::anyhow;
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::{
    auth::{Role, password},
    config::Config,
};

pub async fn run(config: &Config, email: &str, password: &str, role: &str) -> anyhow::Result<()> {
    // The Bun script accepted any string; an unknown role would lock the user out here.
    let role = Role::parse(role)
        .ok_or_else(|| anyhow!("unknown role {role:?}; expected \"admin\" or \"employee\""))?;

    let password_hash = password::hash_password(password.to_owned()).await?;

    let mut conn = PgConnection::connect(&config.database_url).await?;
    let (id, email): (Uuid, String) = sqlx::query_as(
        "INSERT INTO users (email, password_hash, role)
         VALUES ($1, $2, $3)
         ON CONFLICT (email) DO UPDATE
         SET password_hash = EXCLUDED.password_hash, role = EXCLUDED.role
         RETURNING id, email",
    )
    .bind(email)
    .bind(password_hash)
    .bind(role.as_str())
    .fetch_one(&mut conn)
    .await?;
    conn.close().await?;

    tracing::info!(%id, email, role = role.as_str(), "user created/updated");
    Ok(())
}

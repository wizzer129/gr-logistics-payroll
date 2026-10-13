//! Applies `migrations/*.sql`, recording each in `schema_migrations` (the same bookkeeping
//! the original Bun runner used, so databases it migrated are recognized as up to date). Files are embedded in the binary, applied in
//! lexical order, one transaction each, under a Postgres advisory lock so two containers
//! starting at once can't double-apply.

use include_dir::{Dir, include_dir};
use sqlx::{Connection, PgConnection};

use crate::config::Config;

static MIGRATIONS: Dir = include_dir!("$CARGO_MANIFEST_DIR/migrations");

/// Arbitrary but fixed — every migrate run must contend on the same key.
const LOCK_KEY: i64 = 0x6772_6c6f_6769_7374;

pub async fn run(config: &Config) -> anyhow::Result<()> {
    // A dedicated connection, not a pool: the advisory lock belongs to the session, and if
    // anything fails, dropping the connection releases it.
    let mut conn = PgConnection::connect(&config.database_url).await?;
    apply(&mut conn).await?;
    conn.close().await?;
    Ok(())
}

/// Applies pending migrations and returns the names it applied (empty when up to date).
pub async fn apply(conn: &mut PgConnection) -> anyhow::Result<Vec<String>> {
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(LOCK_KEY)
        .execute(&mut *conn)
        .await?;

    let result = apply_locked(conn).await;

    sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(LOCK_KEY)
        .execute(&mut *conn)
        .await?;

    result
}

async fn apply_locked(conn: &mut PgConnection) -> anyhow::Result<Vec<String>> {
    sqlx::raw_sql(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            name text PRIMARY KEY,
            applied_at timestamptz NOT NULL DEFAULT now()
        )",
    )
    .execute(&mut *conn)
    .await?;

    // Read after taking the lock, so a run that waited sees what the other one applied.
    let applied: Vec<String> = sqlx::query_scalar("SELECT name FROM schema_migrations")
        .fetch_all(&mut *conn)
        .await?;

    let mut newly_applied = Vec::new();

    for (name, contents) in pending(&applied)? {
        tracing::info!(file = name, "applying migration");

        let mut tx = conn.begin().await?;
        // raw_sql, not query: migration files hold multiple statements.
        sqlx::raw_sql(contents).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO schema_migrations (name) VALUES ($1)")
            .bind(name)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        newly_applied.push(name.to_owned());
    }

    tracing::info!("migrations up to date");
    Ok(newly_applied)
}

/// Embedded `.sql` files not yet in `applied`, sorted by file name.
fn pending(applied: &[String]) -> anyhow::Result<Vec<(&'static str, &'static str)>> {
    let mut files = Vec::new();

    for file in MIGRATIONS.files() {
        let Some(name) = file.path().to_str() else {
            continue;
        };
        if !name.ends_with(".sql") || applied.iter().any(|a| a == name) {
            continue;
        }
        let contents = file
            .contents_utf8()
            .ok_or_else(|| anyhow::anyhow!("migration {name} is not valid UTF-8"))?;
        files.push((name, contents));
    }

    files.sort_by_key(|(name, _)| *name);
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embeds_migrations_in_order() {
        let names: Vec<_> = pending(&[]).unwrap().into_iter().map(|(n, _)| n).collect();
        assert_eq!(names.first(), Some(&"0001_init.sql"));
        assert!(names.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn skips_applied_migrations() {
        let all = pending(&[]).unwrap();
        let applied = vec!["0001_init.sql".to_owned()];
        let rest = pending(&applied).unwrap();
        assert_eq!(rest.len(), all.len() - 1);
        assert!(rest.iter().all(|(name, _)| *name != "0001_init.sql"));
    }
}

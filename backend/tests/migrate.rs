//! Runs against real Postgres: `#[sqlx::test]` creates a throwaway database per test from
//! DATABASE_URL (loaded via `varlock run` from .env.schema).

use gr_logistics::cli::migrate::apply;
use sqlx::PgPool;

const MIGRATION_COUNT: usize = 9;

#[sqlx::test(migrations = false)]
async fn applies_everything_once_then_nothing(pool: PgPool) {
	let mut conn = pool.acquire().await.unwrap();

	let first = apply(&mut conn).await.unwrap();
	assert_eq!(first.len(), MIGRATION_COUNT);
	assert_eq!(first[0], "0001_init.sql");

	let second = apply(&mut conn).await.unwrap();
	assert!(second.is_empty(), "second run re-applied {second:?}");

	// Final schema reflects every file, e.g. 0004's rename and 0005's column.
	let rate: Option<String> = sqlx::query_scalar(
		"SELECT data_type FROM information_schema.columns
         WHERE table_name = 'employees' AND column_name = 'pay_rate_percent'",
	)
	.fetch_optional(&pool)
	.await
	.unwrap();
	assert_eq!(rate.as_deref(), Some("numeric"));
}

#[sqlx::test(migrations = false)]
async fn concurrent_runs_apply_each_file_exactly_once(pool: PgPool) {
	let run = || async {
		let mut conn = pool.acquire().await.unwrap();
		apply(&mut conn).await.unwrap()
	};

	let (a, b) = tokio::join!(run(), run());
	assert_eq!(a.len() + b.len(), MIGRATION_COUNT);

	let recorded: i64 = sqlx::query_scalar("SELECT count(*) FROM schema_migrations")
		.fetch_one(&pool)
		.await
		.unwrap();
	assert_eq!(recorded, MIGRATION_COUNT as i64);
}

/// A database migrated by the Bun script must look up to date to the Rust runner.
#[sqlx::test(migrations = false)]
async fn recognizes_migrations_applied_by_the_bun_runner(pool: PgPool) {
	let mut conn = pool.acquire().await.unwrap();
	apply(&mut conn).await.unwrap();

	// Simulate the Bun runner's bookkeeping: same table, same file names, different times.
	sqlx::query("UPDATE schema_migrations SET applied_at = now() - interval '30 days'")
		.execute(&pool)
		.await
		.unwrap();

	assert!(apply(&mut conn).await.unwrap().is_empty());
}

//! Hourly cleanup: expired sessions and old reset tokens go; live ones stay.

use gr_logistics::cleanup;
use sqlx::PgPool;

use crate::common::{app_with_db, insert_user};

#[sqlx::test(migrations = false)]
async fn deletes_only_expired_sessions_and_old_reset_tokens(pool: PgPool) {
	let _ = app_with_db(pool.clone()).await; // applies migrations
	let user = insert_user(&pool, "admin@example.com", "admin").await;
	for interval in ["-1 hour", "1 hour"] {
		sqlx::query("INSERT INTO sessions (user_id, expires_at) VALUES ($1, now() + $2::interval)")
			.bind(user)
			.bind(interval)
			.execute(&pool)
			.await
			.unwrap();
	}
	// Expired two days ago (goes), expired an hour ago (kept: the rate limit may count it),
	// and live (kept).
	for (n, interval) in ["-2 days", "-1 hour", "1 hour"].iter().enumerate() {
		sqlx::query(
			"INSERT INTO password_reset_tokens (user_id, token_hash, expires_at)
			 VALUES ($1, $2, now() + $3::interval)",
		)
		.bind(user)
		.bind(vec![n as u8])
		.bind(interval)
		.execute(&pool)
		.await
		.unwrap();
	}

	assert_eq!(cleanup::tick(&pool).await.unwrap(), (1, 1));
	assert_eq!(
		cleanup::tick(&pool).await.unwrap(),
		(0, 0),
		"nothing left to delete"
	);
	let left: (i64, i64) = sqlx::query_as(
		"SELECT (SELECT count(*) FROM sessions), (SELECT count(*) FROM password_reset_tokens)",
	)
	.fetch_one(&pool)
	.await
	.unwrap();
	assert_eq!(left, (1, 2));
}

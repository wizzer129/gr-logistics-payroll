//! Runs against real Postgres (`#[sqlx::test]`, DATABASE_URL from .env.development).

mod common;

use common::{app_with_db, insert_user};
use gr_logistics::services::stores::resolve_store_id;
use sqlx::PgPool;

#[sqlx::test(migrations = false)]
async fn resolves_case_insensitively_and_keeps_first_spelling(pool: PgPool) {
    let _ = app_with_db(pool.clone()).await; // applies migrations
    let user = insert_user(&pool, "admin@example.com", "admin").await;
    let mut conn = pool.acquire().await.unwrap();

    let first = resolve_store_id(&mut conn, "Walmart 12", user)
        .await
        .unwrap();
    let again = resolve_store_id(&mut conn, "  walmart 12 ", user)
        .await
        .unwrap();
    let other = resolve_store_id(&mut conn, "Target 3", user).await.unwrap();

    assert_eq!(first, again);
    assert_ne!(first, other);

    let names: Vec<String> = sqlx::query_scalar("SELECT name FROM stores ORDER BY name")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(names, ["Target 3", "Walmart 12"]);
}

/// Two loads for the same new store, saved at the same moment, must not create duplicates
/// or fail — the race the Bun lookup-then-insert had.
#[sqlx::test(migrations = false)]
async fn concurrent_resolves_create_one_store(pool: PgPool) {
    let _ = app_with_db(pool.clone()).await;
    let user = insert_user(&pool, "admin@example.com", "admin").await;

    let resolve = |name: &'static str| {
        let pool = pool.clone();
        async move {
            let mut conn = pool.acquire().await.unwrap();
            resolve_store_id(&mut conn, name, user).await.unwrap()
        }
    };
    let (a, b) = tokio::join!(resolve("Costco 7"), resolve("COSTCO 7"));
    assert_eq!(a, b);

    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM stores")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[sqlx::test(migrations = false)]
async fn rolls_back_with_the_callers_transaction(pool: PgPool) {
    let _ = app_with_db(pool.clone()).await;
    let user = insert_user(&pool, "admin@example.com", "admin").await;

    let mut tx = pool.begin().await.unwrap();
    resolve_store_id(&mut tx, "Aldi 1", user).await.unwrap();
    tx.rollback().await.unwrap();

    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM stores")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

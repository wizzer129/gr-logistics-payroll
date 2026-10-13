//! Stores are a normalized table, but the API speaks plain `storeName` strings — this maps
//! a typed name to a `store_id`, creating the store on first use.

use sqlx::PgConnection;
use uuid::Uuid;

/// Case-insensitive lookup-or-create in one statement, inside the caller's transaction (the
/// Bun version ran outside it and could race). Relies on `stores_name_lower_key` (0006).
/// The first spelling seen is kept: "walmart 12" after "Walmart 12" returns the same store.
pub async fn resolve_store_id(
    conn: &mut PgConnection,
    name: &str,
    user_id: Uuid,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO stores (name, created_by, last_edited_by)
         VALUES ($1, $2, $2)
         ON CONFLICT (lower(name)) DO UPDATE SET name = stores.name
         RETURNING id",
    )
    .bind(name.trim())
    .bind(user_id)
    .fetch_one(conn)
    .await
}

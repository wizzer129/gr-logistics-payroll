//! Stores are a normalized table, but the API speaks plain `storeName` strings — this maps
//! a typed name to a `store_id`, creating the store on first use.

use sqlx::PgConnection;
use uuid::Uuid;

/// Case-insensitive lookup-or-create for every name, in the caller's transaction so a failed
/// paycheck write leaves no orphan stores; returns ids in the order of `names`. Two
/// statements however many names: one creates the missing stores, the next reads every id.
/// Relies on `stores_name_lower_key` (0006).
///
/// The first spelling seen is kept: "walmart 12" after "Walmart 12" returns the same store,
/// within one call as across calls.
pub async fn resolve_store_ids(
	conn: &mut PgConnection,
	names: &[&str],
	user_id: Uuid,
) -> Result<Vec<Uuid>, sqlx::Error> {
	let names: Vec<&str> = names.iter().map(|name| name.trim()).collect();

	// DO NOTHING waits for a concurrent insert of the same name to commit; the SELECT below
	// is a new statement, so it sees that row.
	sqlx::query(
		"INSERT INTO stores (name, created_by, last_edited_by)
		 SELECT DISTINCT ON (lower(name)) name, $2, $2
		 FROM unnest($1::text[]) WITH ORDINALITY AS input(name, ord)
		 ORDER BY lower(name), ord
		 ON CONFLICT (lower(name)) DO NOTHING",
	)
	.bind(&names)
	.bind(user_id)
	.execute(&mut *conn)
	.await?;

	let ids: Vec<Uuid> = sqlx::query_scalar(
		"SELECT stores.id
		 FROM unnest($1::text[]) WITH ORDINALITY AS input(name, ord)
		 JOIN stores ON lower(stores.name) = lower(input.name)
		 ORDER BY input.ord",
	)
	.bind(&names)
	.fetch_all(&mut *conn)
	.await?;

	// Can't happen (every name was just inserted or already there), but a short list would
	// misattribute loads, so it's an error rather than an assumption.
	if ids.len() != names.len() {
		return Err(sqlx::Error::RowNotFound);
	}
	Ok(ids)
}

/// [`resolve_store_ids`] for one name.
pub async fn resolve_store_id(
	conn: &mut PgConnection,
	name: &str,
	user_id: Uuid,
) -> Result<Uuid, sqlx::Error> {
	Ok(resolve_store_ids(conn, &[name], user_id).await?[0])
}

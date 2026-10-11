import { sql } from '../db/client';

/** Looks up a store by name (case-insensitive) or creates it. Used when recording a load,
 * since the admin types a free-text store name rather than picking from a managed list. */
export async function resolveStoreId(name: string, userId: string): Promise<string> {
	const trimmed = name.trim();

	const [existing] = await sql`SELECT id FROM stores WHERE lower(name) = lower(${trimmed})`;
	if (existing) return existing.id;

	const [created] = await sql`
		INSERT INTO stores (name, created_by, last_edited_by)
		VALUES (${trimmed}, ${userId}, ${userId})
		RETURNING id
	`;
	return created.id;
}

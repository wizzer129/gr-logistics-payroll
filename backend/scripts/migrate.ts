import { readdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { sql } from '../src/db/client';
import { createLogger } from '../src/utils/logger';

const logger = createLogger('migrate');

// The Rust backend owns the schema now (RUST_MIGRATION.md, Phase 1) — this script reads the
// same files so local dev keeps working until cutover. Add new migrations there, not here.
const migrationsDir = join(import.meta.dir, '../../backend-rs/migrations');

async function run(): Promise<void> {
	await sql`
		CREATE TABLE IF NOT EXISTS schema_migrations (
			name text PRIMARY KEY,
			applied_at timestamptz NOT NULL DEFAULT now()
		)
	`;

	const appliedRows = (await sql`SELECT name FROM schema_migrations`) as { name: string }[];
	const applied = new Set(appliedRows.map((row) => row.name));

	const files = (await readdir(migrationsDir)).filter((f) => f.endsWith('.sql')).sort();

	for (const file of files) {
		if (applied.has(file)) continue;

		const contents = await readFile(join(migrationsDir, file), 'utf-8');
		logger.info('applying migration', { file });

		await sql.begin(async (tx) => {
			await tx.unsafe(contents);
			await tx`INSERT INTO schema_migrations (name) VALUES (${file})`;
		});
	}

	logger.info('migrations up to date');
}

await run();

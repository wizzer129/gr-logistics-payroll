/**
 * Phase 7 shadow check (RUST_MIGRATION.md): sends the same read-only requests to the Bun and
 * Rust backends running against the same database, and diffs the JSON. Delete in Phase 8.
 *
 *   BUN_URL=http://localhost:3001 RUST_URL=http://localhost:3002 \
 *   COOKIE='gr_session=<uuid>' bun scripts/shadow-compare.ts
 *
 * Expected, documented differences are normalized away rather than reported:
 *   - dates: Bun sends ISO datetimes, Rust sends "YYYY-MM-DD" (the frontend slices to 10 chars)
 *   - numbers: compared numerically (Rust sends exact decimals like 42.00)
 *   - fields only Rust sends (e.g. `payRatePercent` on paycheck logs) are listed, not failed
 */

const BUN_URL = process.env.BUN_URL ?? 'http://localhost:3001';
const RUST_URL = process.env.RUST_URL ?? 'http://localhost:3002';
const COOKIE = process.env.COOKIE;

if (!COOKIE) {
	console.error('COOKIE=gr_session=<uuid> is required');
	process.exit(1);
}

type Json = null | boolean | number | string | Json[] | { [key: string]: Json };

const ISO_DATETIME = /^(\d{4}-\d{2}-\d{2})T\d{2}:\d{2}:\d{2}(\.\d+)?Z$/;

async function get(base: string, path: string): Promise<{ status: number; body: Json }> {
	const response = await fetch(base + path, { headers: { cookie: COOKIE! } });
	const text = await response.text();
	let body: Json;
	try {
		body = JSON.parse(text);
	} catch {
		body = text;
	}
	return { status: response.status, body };
}

const differences: string[] = [];
const rustOnlyFields = new Set<string>();

function compare(path: string, at: string, bun: Json, rust: Json): void {
	if (typeof bun === 'string' && typeof rust === 'string') {
		const bunDate = bun.match(ISO_DATETIME);
		if (bunDate && /^\d{4}-\d{2}-\d{2}$/.test(rust)) {
			if (bunDate[1] !== rust) differences.push(`${path} ${at}: date ${bun} vs ${rust}`);
			return;
		}
	}
	if (typeof bun === 'number' && typeof rust === 'number') {
		if (Math.abs(bun - rust) > 1e-9) differences.push(`${path} ${at}: ${bun} vs ${rust}`);
		return;
	}
	if (Array.isArray(bun) && Array.isArray(rust)) {
		if (bun.length !== rust.length) {
			differences.push(`${path} ${at}: length ${bun.length} vs ${rust.length}`);
			return;
		}
		// Rust breaks ties by name; Bun's tie order was whatever Postgres returned.
		if (at.endsWith('.byMonth') || at.endsWith('.byEmployee')) {
			const key = (row: Json) => {
				const r = row as Record<string, Json>;
				return `${r.period ?? ''}|${r.employeeId}`;
			};
			bun = [...bun].sort((a, b) => key(a).localeCompare(key(b)));
			rust = [...rust].sort((a, b) => key(a).localeCompare(key(b)));
		}
		bun.forEach((item, index) => compare(path, `${at}[${index}]`, item, rust[index]));
		return;
	}
	if (bun && rust && typeof bun === 'object' && typeof rust === 'object' && !Array.isArray(bun)) {
		const rustObject = rust as Record<string, Json>;
		for (const key of Object.keys(bun)) {
			if (!(key in rustObject)) differences.push(`${path} ${at}.${key}: missing in Rust`);
			else compare(path, `${at}.${key}`, bun[key], rustObject[key]);
		}
		for (const key of Object.keys(rustObject)) {
			if (!(key in bun)) rustOnlyFields.add(key);
		}
		return;
	}
	if (JSON.stringify(bun) !== JSON.stringify(rust)) {
		differences.push(`${path} ${at}: ${JSON.stringify(bun)} vs ${JSON.stringify(rust)}`);
	}
}

async function check(path: string): Promise<Json> {
	const [bun, rust] = await Promise.all([get(BUN_URL, path), get(RUST_URL, path)]);
	if (bun.status !== rust.status) {
		differences.push(`${path}: status ${bun.status} vs ${rust.status}`);
	} else {
		compare(path, '$', bun.body, rust.body);
	}
	return rust.body;
}

const paths: string[] = [];
async function run(path: string): Promise<Json> {
	paths.push(path);
	return check(path);
}

await run('/api/auth/me');
const employees = (await run('/api/employees')) as { id: string }[];
const logs = (await run('/api/paycheck-logs')) as { id: string; weekStart: string }[];

const years = new Set(logs.map((log) => Number(log.weekStart.slice(0, 4))));
for (const year of [...years].sort()) {
	await run(`/api/dashboard/summary?year=${year}`);
	for (const employee of employees) {
		await run(`/api/dashboard/employee/${employee.id}/summary?year=${year}`);
	}
}
for (const employee of employees) {
	await run(`/api/paycheck-logs?employeeId=${employee.id}`);
}
for (const log of logs) {
	await run(`/api/paycheck-logs/${log.id}/loads`);
}

console.log(`compared ${paths.length} requests (${logs.length} logs, ${employees.length} employees)`);
if (rustOnlyFields.size > 0) {
	console.log(`fields only Rust sends (expected additions): ${[...rustOnlyFields].join(', ')}`);
}
if (differences.length === 0) {
	console.log('no differences');
} else {
	console.log(`${differences.length} differences:`);
	for (const line of differences.slice(0, 40)) console.log(`  ${line}`);
	if (differences.length > 40) console.log(`  … ${differences.length - 40} more`);
	process.exit(1);
}

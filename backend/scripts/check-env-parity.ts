import { readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';

function keysOf(path: string): Set<string> {
	if (!existsSync(path)) return new Set();

	return new Set(
		readFileSync(path, 'utf-8')
			.split('\n')
			.map((line) => line.trim())
			.filter((line) => line && !line.startsWith('#'))
			.map((line) => line.split('=')[0]),
	);
}

const root = join(import.meta.dir, '..');
const exampleKeys = keysOf(join(root, '.env.example'));

const targets = ['.env.development', '.env.production'];
let failed = false;

for (const target of targets) {
	const targetPath = join(root, target);

	if (!existsSync(targetPath)) {
		console.log(
			`skip: ${target} not present (fine in CI, required on the server for production)`,
		);
		continue;
	}

	const targetKeys = keysOf(targetPath);
	const missing = [...exampleKeys].filter((key) => !targetKeys.has(key));

	if (missing.length > 0) {
		failed = true;
		console.error(`${target} is missing keys present in .env.example: ${missing.join(', ')}`);
	}
}

if (failed) process.exit(1);

console.log('env parity check passed');

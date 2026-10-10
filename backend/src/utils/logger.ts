const LEVELS = ['ERROR', 'WARN', 'INFO', 'DEBUG'] as const;

type Level = (typeof LEVELS)[number];

const activeLevel = (process.env.LOG_LEVEL?.toUpperCase() as Level) ?? 'INFO';
const activeIndex = LEVELS.includes(activeLevel)
	? LEVELS.indexOf(activeLevel)
	: LEVELS.indexOf('INFO');

// JSON for log collectors in production; human-readable lines in development.
// LOG_FORMAT=json|pretty overrides the NODE_ENV default.
const format =
	process.env.LOG_FORMAT?.toLowerCase() ??
	(process.env.NODE_ENV === 'development' ? 'pretty' : 'json');

const useColor =
	!process.env.NO_COLOR && (Boolean(process.env.FORCE_COLOR) || Boolean(process.stdout.isTTY));

const LEVEL_COLORS: Record<Level, string> = {
	ERROR: '\x1b[31m',
	WARN: '\x1b[33m',
	INFO: '\x1b[32m',
	DEBUG: '\x1b[34m',
};
const DIM = '\x1b[2m';
const RESET = '\x1b[0m';

const paint = (color: string, text: string) => (useColor ? `${color}${text}${RESET}` : text);

// One distinct color per log scope (file/module), so output from different
// sources is visually separable in a dev console. Assignment is a stable hash
// of the scope name, not random, so a given scope is always the same color.
// A wide 256-color palette (rather than the basic 8/16 ANSI colors) keeps
// collisions rare even as the number of distinct scopes in the codebase grows.
const SCOPE_COLOR_CODES = [
	39, 45, 51, 77, 83, 99, 105, 111, 117, 118, 123, 129, 135, 141, 147, 148, 154, 160, 166, 171,
	172, 178, 184, 190, 196, 202, 208, 209, 213, 214, 220, 226,
];

const scopeColors = new Map<string, string>();

/** FNV-1a — distributes short, similar strings (file names) across the palette better than a naive hash. */
function hashScope(scope: string): number {
	let hash = 0x811c9dc5;
	for (let i = 0; i < scope.length; i++) {
		hash ^= scope.charCodeAt(i);
		hash = Math.imul(hash, 0x01000193);
	}
	return hash >>> 0;
}

function colorForScope(scope: string): string {
	const cached = scopeColors.get(scope);
	if (cached) return cached;

	const code = SCOPE_COLOR_CODES[hashScope(scope) % SCOPE_COLOR_CODES.length];
	const color = `\x1b[38;5;${code}m`;
	scopeColors.set(scope, color);
	return color;
}

function formatValue(value: unknown): string {
	if (typeof value === 'string') return /\s/.test(value) ? JSON.stringify(value) : value;
	if (value instanceof Error) return JSON.stringify(value.stack ?? value.message);
	return JSON.stringify(value);
}

function formatPretty(
	level: Level,
	scope: string,
	message: string,
	meta: Record<string, unknown>,
): string {
	const time = new Date().toISOString().slice(11, 23);
	const fields = Object.entries(meta)
		.map(([key, value]) => `${paint(DIM, `${key}=`)}${formatValue(value)}`)
		.join(' ');

	return [
		paint(DIM, time),
		paint(LEVEL_COLORS[level], level.padEnd(5)),
		paint(colorForScope(scope), `[${scope}]`),
		message,
		fields,
	]
		.filter(Boolean)
		.join(' ');
}

function log(
	level: Level,
	scope: string,
	message: string,
	meta: Record<string, unknown> = {},
): void {
	if (LEVELS.indexOf(level) > activeIndex) return;

	const line =
		format === 'pretty'
			? formatPretty(level, scope, message, meta)
			: JSON.stringify({
					level,
					scope,
					message,
					time: new Date().toISOString(),
					...meta,
				});

	if (level === 'ERROR') console.error(line);
	else console.log(line);
}

export type Logger = {
	error: (message: string, meta?: Record<string, unknown>) => void;
	warn: (message: string, meta?: Record<string, unknown>) => void;
	info: (message: string, meta?: Record<string, unknown>) => void;
	debug: (message: string, meta?: Record<string, unknown>) => void;
};

/** A logger tagged with a scope (typically the file/module name) that's shown, in a
 * color unique to that scope, on every line it logs — e.g. createLogger('auth-service'). */
export function createLogger(scope: string): Logger {
	return {
		error: (message, meta) => log('ERROR', scope, message, meta),
		warn: (message, meta) => log('WARN', scope, message, meta),
		info: (message, meta) => log('INFO', scope, message, meta),
		debug: (message, meta) => log('DEBUG', scope, message, meta),
	};
}

const LEVELS = ['ERROR', 'WARN', 'INFO', 'DEBUG'] as const;

type Level = (typeof LEVELS)[number];

const activeLevel = (process.env.LOG_LEVEL?.toUpperCase() as Level) ?? 'INFO';
const activeIndex = LEVELS.includes(activeLevel)
	? LEVELS.indexOf(activeLevel)
	: LEVELS.indexOf('INFO');

function log(level: Level, message: string, meta: Record<string, unknown> = {}): void {
	if (LEVELS.indexOf(level) > activeIndex) return;

	const line = JSON.stringify({
		level,
		message,
		time: new Date().toISOString(),
		...meta,
	});

	if (level === 'ERROR') console.error(line);
	else console.log(line);
}

export const logger = {
	error: (message: string, meta?: Record<string, unknown>) => log('ERROR', message, meta),
	warn: (message: string, meta?: Record<string, unknown>) => log('WARN', message, meta),
	info: (message: string, meta?: Record<string, unknown>) => log('INFO', message, meta),
	debug: (message: string, meta?: Record<string, unknown>) => log('DEBUG', message, meta),
};

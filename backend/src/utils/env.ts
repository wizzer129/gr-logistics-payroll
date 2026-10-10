import { logger } from './logger';

const REQUIRED_KEYS = [
	'NODE_ENV',
	'PORT',
	'DATABASE_URL',
	'SESSION_SECRET',
	'SESSION_TTL_HOURS',
	'ANTHROPIC_API_KEY',
	'LOG_LEVEL',
] as const;

export function assertEnv(): void {
	const missing = REQUIRED_KEYS.filter((key) => !process.env[key]);

	if (missing.length > 0) {
		logger.error('Missing required environment variables', { missing });
		throw new Error(`Missing required environment variables: ${missing.join(', ')}`);
	}
}

export const env = {
	nodeEnv: process.env.NODE_ENV ?? 'development',
	port: Number(process.env.PORT ?? 3000),
	databaseUrl: process.env.DATABASE_URL ?? '',
	sessionSecret: process.env.SESSION_SECRET ?? '',
	sessionTtlHours: Number(process.env.SESSION_TTL_HOURS ?? 12),
	anthropicApiKey: process.env.ANTHROPIC_API_KEY ?? '',
};

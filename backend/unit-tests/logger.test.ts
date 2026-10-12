import { afterEach, describe, expect, spyOn, test } from 'bun:test';
import { createLogger } from '../src/utils/logger';

describe('createLogger', () => {
	afterEach(() => {
		(console.error as unknown as { mockRestore?: () => void }).mockRestore?.();
	});

	// Uses .error() rather than .info() — the active log level is read from LOG_LEVEL once
	// at module load, so under CI's LOG_LEVEL=ERROR an .info() call is silently dropped and
	// this spy never fires. ERROR is always emitted regardless of LOG_LEVEL.
	test('tags each line with the scope it was created for', () => {
		const spy = spyOn(console, 'error').mockImplementation(() => {});

		createLogger('auth-service').error('login failed', { userId: 'abc' });
		createLogger('migrate').error('migration failed', { file: '0001_init.sql' });

		const authLine = JSON.parse(spy.mock.calls[0][0] as string);
		const migrateLine = JSON.parse(spy.mock.calls[1][0] as string);

		expect(authLine.scope).toBe('auth-service');
		expect(authLine.message).toBe('login failed');
		expect(migrateLine.scope).toBe('migrate');
	});
});

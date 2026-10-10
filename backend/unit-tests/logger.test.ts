import { afterEach, describe, expect, spyOn, test } from 'bun:test';
import { createLogger } from '../src/utils/logger';

describe('createLogger', () => {
	afterEach(() => {
		(console.log as unknown as { mockRestore?: () => void }).mockRestore?.();
	});

	test('tags each line with the scope it was created for', () => {
		const spy = spyOn(console, 'log').mockImplementation(() => {});

		createLogger('auth-service').info('user logged in', { userId: 'abc' });
		createLogger('migrate').info('applying migration', { file: '0001_init.sql' });

		const authLine = JSON.parse(spy.mock.calls[0][0] as string);
		const migrateLine = JSON.parse(spy.mock.calls[1][0] as string);

		expect(authLine.scope).toBe('auth-service');
		expect(authLine.message).toBe('user logged in');
		expect(migrateLine.scope).toBe('migrate');
	});
});

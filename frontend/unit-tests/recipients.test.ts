import { describe, expect, test } from 'vitest';
import { fromKeys, toKeys } from '../src/utils/recipients';

describe('recipient keys', () => {
	test('round-trip users and employees through one list of keys', () => {
		const selection = { userIds: ['u1', 'u2'], employeeIds: ['e1'] };
		const keys = toKeys(selection);
		expect(keys).toEqual(['user:u1', 'user:u2', 'employee:e1']);
		expect(fromKeys(keys)).toEqual(selection);
	});

	test('ignores keys of an unknown kind', () => {
		expect(fromKeys(['user:u1', 'other:x'])).toEqual({ userIds: ['u1'], employeeIds: [] });
	});
});

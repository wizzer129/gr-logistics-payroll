import { describe, expect, test } from 'vitest';
import { fromKeys, isEmail, recipientCount, toKeys } from '../src/utils/recipients';

describe('recipient keys', () => {
	test('round-trip users, employees and typed addresses through one list of keys', () => {
		const selection = {
			userIds: ['u1', 'u2'],
			employeeIds: ['e1'],
			emails: ['office@example.org'],
		};
		const keys = toKeys(selection);
		expect(keys).toEqual(['user:u1', 'user:u2', 'employee:e1', 'office@example.org']);
		expect(fromKeys(keys)).toEqual(selection);
		expect(recipientCount(selection)).toBe(4);
	});

	test('ignores keys of an unknown kind', () => {
		expect(fromKeys(['user:u1', 'other:x'])).toEqual({
			userIds: ['u1'],
			employeeIds: [],
			emails: [],
		});
	});

	test('trims typed addresses', () => {
		expect(fromKeys(['  a@b.co ']).emails).toEqual(['a@b.co']);
	});
});

describe('isEmail', () => {
	test.each(['a@b.co', 'first.last+tag@example.org'])('accepts %s', (value) => {
		expect(isEmail(value)).toBe(true);
	});

	test.each(['', 'nope', 'a@b', 'a b@c.co', '@b.co'])('rejects %j', (value) => {
		expect(isEmail(value)).toBe(false);
	});
});

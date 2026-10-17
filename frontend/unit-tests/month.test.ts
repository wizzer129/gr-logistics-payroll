import { describe, expect, test } from 'vitest';
import { currentMonth, previousMonth } from '../src/utils/month';

describe('month', () => {
	test('previous month, across a year boundary', () => {
		expect(previousMonth(new Date(2026, 9, 16))).toBe('2026-09');
		expect(previousMonth(new Date(2026, 0, 1))).toBe('2025-12');
		expect(previousMonth(new Date(2026, 2, 31))).toBe('2026-02');
	});

	test('current month', () => {
		expect(currentMonth(new Date(2026, 9, 16))).toBe('2026-10');
	});
});

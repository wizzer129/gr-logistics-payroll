import { describe, expect, test } from 'vitest';
import { getWeekRange } from '../src/utils/week';

describe('getWeekRange', () => {
	test('a mid-week date resolves to its Sunday-Saturday week', () => {
		// Wednesday, Oct 7 2026
		expect(getWeekRange('2026-10-07')).toEqual({
			weekStart: '2026-10-04',
			weekEnd: '2026-10-10',
		});
	});

	test('selecting the Sunday itself resolves to that same week', () => {
		expect(getWeekRange('2026-10-04')).toEqual({
			weekStart: '2026-10-04',
			weekEnd: '2026-10-10',
		});
	});

	test('selecting the Saturday itself resolves to that same week', () => {
		expect(getWeekRange('2026-10-10')).toEqual({
			weekStart: '2026-10-04',
			weekEnd: '2026-10-10',
		});
	});

	test('handles a week that spans a month boundary', () => {
		// Friday, Oct 30 2026 -> week runs Oct 25 - Oct 31
		expect(getWeekRange('2026-10-30')).toEqual({
			weekStart: '2026-10-25',
			weekEnd: '2026-10-31',
		});
	});
});

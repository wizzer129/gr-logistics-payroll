import { describe, expect, test } from 'vitest';
import { formatMonthDay, getWeekRange } from '../src/utils/week';

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

describe('formatMonthDay', () => {
	test('formats a plain date as "Month Day"', () => {
		expect(formatMonthDay('2026-01-04', 'en')).toBe('January 4');
	});

	test('formats a full ISO datetime (as returned by the API) the same way', () => {
		expect(formatMonthDay('2026-01-04T00:00:00.000Z', 'en')).toBe('January 4');
	});

	test('never shifts a day backward across timezones', () => {
		// A UTC-midnight timestamp is the classic trap: new Date(iso) in a negative-offset
		// timezone would render as the previous day. formatMonthDay must not do that.
		expect(formatMonthDay('2026-01-01T00:00:00.000Z', 'en')).toBe('January 1');
	});
});

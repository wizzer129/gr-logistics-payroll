/** Parses a "YYYY-MM-DD" string as a local date (avoids the UTC-midnight shift new Date(iso) causes). */
export function parseLocalDate(isoDate: string): Date {
	const [year, month, day] = isoDate.split('-').map(Number);
	return new Date(year, month - 1, day);
}

export function toIsoDate(date: Date): string {
	const year = date.getFullYear();
	const month = String(date.getMonth() + 1).padStart(2, '0');
	const day = String(date.getDate()).padStart(2, '0');
	return `${year}-${month}-${day}`;
}

/** Formats an API date — plain "YYYY-MM-DD" or a full ISO datetime ("...T00:00:00.000Z") —
 * as "Month Day" (e.g. "January 4") in the given locale, parsed as a local calendar date
 * so it never shifts a day backward/forward across timezones. */
export function formatMonthDay(isoDateOrDateTime: string, locale: string): string {
	const date = parseLocalDate(isoDateOrDateTime.slice(0, 10));
	return new Intl.DateTimeFormat(locale, { month: 'long', day: 'numeric' }).format(date);
}

export type WeekRange = { weekStart: string; weekEnd: string };

/** Sunday–Saturday week containing the given date. */
export function getWeekRange(isoDate: string): WeekRange {
	const date = parseLocalDate(isoDate);

	const weekStart = new Date(date);
	weekStart.setDate(date.getDate() - date.getDay());

	const weekEnd = new Date(weekStart);
	weekEnd.setDate(weekStart.getDate() + 6);

	return { weekStart: toIsoDate(weekStart), weekEnd: toIsoDate(weekEnd) };
}

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

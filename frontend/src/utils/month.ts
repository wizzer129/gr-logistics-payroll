// Months as `YYYY-MM` (what `<input type="month">` and the monthly report API use), in local time.
export function monthKey(date: Date): string {
	return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}`;
}

export function currentMonth(today: Date = new Date()): string {
	return monthKey(today);
}

/** The monthly report's default: last month. */
export function previousMonth(today: Date = new Date()): string {
	return monthKey(new Date(today.getFullYear(), today.getMonth() - 1, 1));
}

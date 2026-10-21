/** One row of Payroll History: a paycheck log with its employee's name. */
export type PaycheckLogRow = {
	id: string;
	employeeId: string;
	weekStart: string;
	weekEnd: string;
	payday: string;
	status: 'draft' | 'completed';
	/** `loads` or `lump_sum` (no loads, paid in full). */
	kind: 'loads' | 'lump_sum';
	summaryTotal: number;
	employeeCut: number;
	// The rate this log's cut was computed at; the backend recomputes edits at it too.
	payRatePercent: number;
	firstName: string;
	lastName: string;
};

/** Only completed logs are pay stubs, so only they can be emailed (or picked to be). */
export function canEmail(log: PaycheckLogRow): boolean {
	return log.status === 'completed';
}

/** `ids` with `id` added, or removed if it was there. */
export function toggleId(ids: string[], id: string): string[] {
	return ids.includes(id) ? ids.filter((other) => other !== id) : [...ids, id];
}

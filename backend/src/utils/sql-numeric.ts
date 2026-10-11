/** Postgres `numeric`/`decimal` columns come back from Bun's SQL driver as strings
 * (precision-safe, but vue-i18n's `n()` throws "Invalid arguments" on a non-number) —
 * convert at the API boundary so every currency field the frontend receives is a real
 * JS number. Never use this on an actual bigint-range value; this app's money columns
 * are all numeric(10,2), far under Number's safe-integer range. */
export function toNum(value: unknown): number {
	return Number(value);
}

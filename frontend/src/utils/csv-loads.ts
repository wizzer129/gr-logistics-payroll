import type { Load } from '../store/payment-processing-store';
import { calculateEmployeeCut } from './payment-calc';

/**
 * Parses a weekly loads CSV (see repo-root `csv/` for samples):
 *
 *   Freddy Alexander Corrales,10/16/26      <- employee name, payday
 *   Date,Load,Pay,Extra Pay                 <- header
 *   10/5/26,#42,336,                        <- one row per load
 *   Total,,4846,                            <- the file's own total (optional)
 *   42%,,2034,                              <- the file's own employee cut (optional)
 *
 * The two summary rows are only read to check against what the site computes from the
 * loads (`checkCsvTotals`) — they never feed into the paycheck.
 */

export type ParsedLoadsCsv = {
	employeeName: string;
	/** "YYYY-MM-DD" */
	payday: string;
	loads: Load[];
	/** The "Total" row's amount, or null if the file has none (or it isn't a number). */
	csvTotal: number | null;
	/** The percent row's amount (e.g. "42%,,2034"), or null if the file has none. */
	csvTotalPay: number | null;
};

export type CsvErrorCode =
	| 'empty'
	| 'missingNameOrPayday'
	| 'badPayday'
	| 'missingHeader'
	| 'badDate'
	| 'missingStore'
	| 'badPay'
	| 'badExtraPay'
	| 'noLoads';

export class CsvLoadsError extends Error {
	/** 1-based line in the file, when the error is about a specific row. */
	readonly line?: number;
	readonly code: CsvErrorCode;

	constructor(code: CsvErrorCode, line?: number) {
		super(line ? `${code} (line ${line})` : code);
		this.name = 'CsvLoadsError';
		this.code = code;
		this.line = line;
	}
}

const EXPECTED_HEADER = ['date', 'load', 'pay', 'extra pay'];

export function parseLoadsCsv(text: string): ParsedLoadsCsv {
	const rows = text
		.replace(/^\uFEFF/, '')
		.split(/\r?\n/)
		.map((line, index) => ({ line: index + 1, cells: splitCsvLine(line) }))
		.filter((row) => row.cells.some((cell) => cell !== ''));

	if (rows.length === 0) throw new CsvLoadsError('empty');

	const [first, header, ...body] = rows;
	const employeeName = first.cells[0];
	if (!employeeName || !first.cells[1])
		throw new CsvLoadsError('missingNameOrPayday', first.line);
	const payday = parseUsDate(first.cells[1]);
	if (!payday) throw new CsvLoadsError('badPayday', first.line);

	const headerCells = header?.cells.slice(0, EXPECTED_HEADER.length).map((c) => c.toLowerCase());
	if (!headerCells || EXPECTED_HEADER.some((name, i) => headerCells[i] !== name)) {
		throw new CsvLoadsError('missingHeader', header?.line ?? first.line + 1);
	}

	const loads: Load[] = [];
	let summaryStart = body.length;
	for (const [index, { line, cells }] of body.entries()) {
		const [rawDate = '', storeName = '', rawPay = '', rawExtraPay = ''] = cells;
		if (rawDate.toLowerCase() === 'total') {
			summaryStart = index;
			break;
		}

		const loadDate = parseUsDate(rawDate);
		if (!loadDate) throw new CsvLoadsError('badDate', line);
		if (!storeName) throw new CsvLoadsError('missingStore', line);
		const pay = parseMoney(rawPay);
		if (pay === null) throw new CsvLoadsError('badPay', line);
		const extraPay = rawExtraPay === '' ? 0 : parseMoney(rawExtraPay);
		if (extraPay === null) throw new CsvLoadsError('badExtraPay', line);

		loads.push({ storeName, loadDate, pay, extraPay });
	}

	if (loads.length === 0) throw new CsvLoadsError('noLoads');

	const summary = body.slice(summaryStart);
	const totalRow = summary.find((row) => row.cells[0]?.toLowerCase() === 'total');
	const percentRow = summary.find((row) => /^\d+(\.\d+)?%$/.test(row.cells[0] ?? ''));

	return {
		employeeName,
		payday,
		loads,
		csvTotal: parseMoney(totalRow?.cells[2] ?? ''),
		csvTotalPay: parseMoney(percentRow?.cells[2] ?? ''),
	};
}

export type CsvTotalCheck = {
	/** What the site computes from the file's loads. */
	expected: number;
	/** False when the file's amount differs; null when the file doesn't have that row. */
	matches: boolean | null;
};

/** Compares the file's Total and percent rows with the site's own summary total and pay. */
export function checkCsvTotals(
	csv: ParsedLoadsCsv,
	payRatePercent: number,
): { total: CsvTotalCheck; totalPay: CsvTotalCheck } {
	const total = csv.loads.reduce((sum, l) => sum + l.pay + (l.extraPay ?? 0), 0);
	const totalPay = calculateEmployeeCut(total, payRatePercent).roundedPay;
	return {
		total: { expected: total, matches: sameAmount(csv.csvTotal, total) },
		totalPay: { expected: totalPay, matches: sameAmount(csv.csvTotalPay, totalPay) },
	};
}

/** Compared in cents, so float noise in the sum (0.1 + 0.2) never reads as a mismatch. */
function sameAmount(fromCsv: number | null, computed: number): boolean | null {
	if (fromCsv === null) return null;
	return Math.round(fromCsv * 100) === Math.round(computed * 100);
}

/** Splits one CSV line, honoring double-quoted cells ("1,234" and "" escapes). Cells are trimmed. */
function splitCsvLine(line: string): string[] {
	const cells: string[] = [];
	let cell = '';
	let quoted = false;

	for (let i = 0; i < line.length; i++) {
		const char = line[i];
		if (quoted) {
			if (char === '"' && line[i + 1] === '"') {
				cell += '"';
				i++;
			} else if (char === '"') {
				quoted = false;
			} else {
				cell += char;
			}
		} else if (char === '"') {
			quoted = true;
		} else if (char === ',') {
			cells.push(cell.trim());
			cell = '';
		} else {
			cell += char;
		}
	}
	cells.push(cell.trim());
	return cells;
}

/** "M/D/YY" or "M/D/YYYY" -> "YYYY-MM-DD"; null if it isn't a real calendar date. */
function parseUsDate(value: string): string | null {
	const match = /^(\d{1,2})\/(\d{1,2})\/(\d{2}|\d{4})$/.exec(value);
	if (!match) return null;

	const month = Number(match[1]);
	const day = Number(match[2]);
	const year = match[3].length === 2 ? 2000 + Number(match[3]) : Number(match[3]);

	const date = new Date(year, month - 1, day);
	if (date.getFullYear() !== year || date.getMonth() !== month - 1 || date.getDate() !== day) {
		return null;
	}
	return `${year}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
}

/** "336", "$1,234.50" -> number; null if not a non-negative amount with at most 2 decimals. */
function parseMoney(value: string): number | null {
	const cleaned = value.replace(/[$,]/g, '');
	if (!/^\d+(\.\d{1,2})?$/.test(cleaned)) return null;
	return Number(cleaned);
}

type NamedEmployee = { id: string; firstName: string; lastName: string };

/**
 * Finds the employee a CSV's name refers to. An exact full-name match wins; otherwise a
 * unique last-name match (the files spell first names loosely — "Freddy" vs "Fredy").
 * Returns null when nothing or more than one employee matches, so the user picks manually.
 */
export function matchEmployeeByName<T extends NamedEmployee>(
	name: string,
	employees: T[],
): T | null {
	const target = normalizeName(name);
	const exact = employees.filter((e) => normalizeName(`${e.firstName} ${e.lastName}`) === target);
	if (exact.length === 1) return exact[0];

	const targetWords = new Set(target.split(' '));
	const byLastName = employees.filter((e) => {
		const lastNameWords = normalizeName(e.lastName).split(' ').filter(Boolean);
		return lastNameWords.length > 0 && lastNameWords.every((word) => targetWords.has(word));
	});
	return byLastName.length === 1 ? byLastName[0] : null;
}

function normalizeName(name: string): string {
	return name.normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase().replace(/\s+/g, ' ').trim();
}

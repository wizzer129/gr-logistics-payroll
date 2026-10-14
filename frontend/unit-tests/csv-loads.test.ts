import { describe, expect, test } from 'vitest';
import {
	checkCsvTotals,
	CsvLoadsError,
	matchEmployeeByName,
	parseLoadsCsv,
} from '../src/utils/csv-loads';

const SAMPLE = `Freddy Alexander Corrales,10/16/26
Date,Load,Pay,Extra Pay
10/5/26,#42,336,
10/7/26,Frida,378,150
10/9/26,Palmdale Fontana,200,
Total,,1064,
42%,,447,
`;

function errorCode(text: string): string | undefined {
	try {
		parseLoadsCsv(text);
	} catch (e) {
		if (e instanceof CsvLoadsError) return e.line ? `${e.code}@${e.line}` : e.code;
		throw e;
	}
	return undefined;
}

describe('parseLoadsCsv', () => {
	test('reads the employee name, payday and every load row', () => {
		expect(parseLoadsCsv(SAMPLE)).toEqual({
			employeeName: 'Freddy Alexander Corrales',
			payday: '2026-10-16',
			loads: [
				{ storeName: '#42', loadDate: '2026-10-05', pay: 336, extraPay: 0 },
				{ storeName: 'Frida', loadDate: '2026-10-07', pay: 378, extraPay: 150 },
				{ storeName: 'Palmdale Fontana', loadDate: '2026-10-09', pay: 200, extraPay: 0 },
			],
			csvTotal: 1064,
			csvTotalPay: 447,
		});
	});

	test('a file without a percent row has no CSV total pay', () => {
		const parsed = parseLoadsCsv(SAMPLE.replace('42%,,447,\n', ''));
		expect(parsed.loads).toHaveLength(3);
		expect(parsed.csvTotal).toBe(1064);
		expect(parsed.csvTotalPay).toBeNull();
	});

	test('a file without summary rows has neither CSV total', () => {
		const parsed = parseLoadsCsv(SAMPLE.replace(/Total[\s\S]*/, ''));
		expect(parsed.loads).toHaveLength(3);
		expect(parsed.csvTotal).toBeNull();
		expect(parsed.csvTotalPay).toBeNull();
	});

	test('accepts CRLF line endings, a BOM, quoted money and 4-digit years', () => {
		const text =
			'\uFEFFPablo Ramirez,10/16/2026\r\nDate,Load,Pay,Extra Pay\r\n10/4/2026,"AMS, North","$1,324.50",\r\n';
		expect(parseLoadsCsv(text)).toEqual({
			employeeName: 'Pablo Ramirez',
			payday: '2026-10-16',
			loads: [{ storeName: 'AMS, North', loadDate: '2026-10-04', pay: 1324.5, extraPay: 0 }],
			csvTotal: null,
			csvTotalPay: null,
		});
	});

	test('rejects bad input with the offending line', () => {
		expect(errorCode('')).toBe('empty');
		expect(errorCode('Pablo Ramirez\n')).toBe('missingNameOrPayday@1');
		expect(errorCode('Pablo Ramirez,13/40/26\n')).toBe('badPayday@1');
		expect(errorCode('Pablo Ramirez,10/16/26\n10/4/26,AMS,324,\n')).toBe('missingHeader@2');
		expect(errorCode(SAMPLE.replace('10/5/26', '2026-10-05'))).toBe('badDate@3');
		expect(errorCode(SAMPLE.replace('#42', ''))).toBe('missingStore@3');
		expect(errorCode(SAMPLE.replace('336', 'abc'))).toBe('badPay@3');
		expect(errorCode(SAMPLE.replace(',150', ',-5'))).toBe('badExtraPay@4');
		expect(errorCode('Pablo Ramirez,10/16/26\nDate,Load,Pay,Extra Pay\nTotal,,0,\n')).toBe(
			'noLoads',
		);
	});
});

describe('checkCsvTotals', () => {
	test('matching totals pass', () => {
		// 1064 * 42% = 446.88 -> $447
		expect(checkCsvTotals(parseLoadsCsv(SAMPLE), 42)).toEqual({
			total: { expected: 1064, matches: true },
			totalPay: { expected: 447, matches: true },
		});
	});

	test('a total or pay that differs from the site’s calculation is flagged', () => {
		const csv = parseLoadsCsv(
			SAMPLE.replace('Total,,1064', 'Total,,1000').replace(',,447', ',,446'),
		);
		const result = checkCsvTotals(csv, 42);
		expect(result.total.matches).toBe(false);
		expect(result.totalPay.matches).toBe(false);
	});

	test('total pay is checked at the given rate', () => {
		// 1064 * 40% = 425.6 -> $426, not the file’s $447
		expect(checkCsvTotals(parseLoadsCsv(SAMPLE), 40).totalPay).toEqual({
			expected: 426,
			matches: false,
		});
	});

	test('a row missing from the file is neither a match nor a mismatch', () => {
		const csv = parseLoadsCsv(SAMPLE.replace(/Total[\s\S]*/, ''));
		const result = checkCsvTotals(csv, 42);
		expect(result.total.matches).toBeNull();
		expect(result.totalPay.matches).toBeNull();
	});

	test('cents are compared exactly, without float noise', () => {
		const text = 'A B,10/16/26\nDate,Load,Pay,Extra Pay\n10/4/26,X,0.1,0.2\nTotal,,0.30,\n';
		expect(checkCsvTotals(parseLoadsCsv(text), 42).total.matches).toBe(true);
	});
});

describe('matchEmployeeByName', () => {
	const employees = [
		{ id: '1', firstName: 'Fredy Alexander', lastName: 'Corrales' },
		{ id: '2', firstName: 'Mauricio', lastName: 'Sanchez' },
		{ id: '3', firstName: 'Pablo', lastName: 'Ramírez' },
		{ id: '4', firstName: 'Juan', lastName: 'Lopez' },
		{ id: '5', firstName: 'Maria', lastName: 'Lopez' },
	];

	test('exact full name, ignoring case and accents', () => {
		expect(matchEmployeeByName('pablo ramirez', employees)?.id).toBe('3');
	});

	test('falls back to a unique last-name match when the first name is spelled differently', () => {
		expect(matchEmployeeByName('Freddy Alexander Corrales', employees)?.id).toBe('1');
		expect(matchEmployeeByName('Moastisio Sanchez', employees)?.id).toBe('2');
	});

	test('no match, or an ambiguous last name, returns null', () => {
		expect(matchEmployeeByName('Juan Carlos Ibarra', employees)).toBeNull();
		expect(matchEmployeeByName('Pedro Lopez', employees)).toBeNull();
	});
});

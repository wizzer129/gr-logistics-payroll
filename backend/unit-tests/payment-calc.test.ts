import { describe, expect, test } from 'bun:test';
import { calculateEmployeeCut, roundToDollar, sumLoads } from '../src/utils/payment-calc';

describe('roundToDollar', () => {
	test('rounds .50 up', () => {
		expect(roundToDollar(120.5)).toBe(121);
	});

	test('rounds .49 down', () => {
		expect(roundToDollar(120.49)).toBe(120);
	});

	test('handles zero', () => {
		expect(roundToDollar(0)).toBe(0);
	});
});

describe('calculateEmployeeCut', () => {
	test('applies 42% cut and rounds', () => {
		const { employeeCut, roundedPay } = calculateEmployeeCut(1000);
		expect(employeeCut).toBeCloseTo(420, 5);
		expect(roundedPay).toBe(420);
	});
});

describe('sumLoads', () => {
	test('sums pay and extra pay across loads', () => {
		const total = sumLoads([
			{ pay: 100, extraPay: 10 },
			{ pay: 50, extraPay: 0 },
		]);
		expect(total).toBe(160);
	});
});

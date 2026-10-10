import { describe, expect, test } from 'vitest';
import { calculateEmployeeCut, roundToDollar } from '../src/utils/payment-calc';

describe('roundToDollar', () => {
	test('rounds .50 up', () => {
		expect(roundToDollar(120.5)).toBe(121);
	});

	test('rounds .49 down', () => {
		expect(roundToDollar(120.49)).toBe(120);
	});
});

describe('calculateEmployeeCut', () => {
	test('applies 42% cut and rounds', () => {
		const { roundedPay } = calculateEmployeeCut(1000);
		expect(roundedPay).toBe(420);
	});
});

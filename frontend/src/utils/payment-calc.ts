// Fallback only — not every employee is cut at this rate (see employees.pay_rate_percent).
export const DEFAULT_PAY_RATE_PERCENT = 42;

/** Round-half-up to the nearest whole dollar: .50+ rounds up, .49 and below rounds down. */
export function roundToDollar(amount: number): number {
	return Math.floor(amount + 0.5);
}

export function calculateEmployeeCut(
	summaryTotal: number,
	payRatePercent: number = DEFAULT_PAY_RATE_PERCENT,
): {
	employeeCut: number;
	roundedPay: number;
} {
	const employeeCut = summaryTotal * (payRatePercent / 100);
	return { employeeCut, roundedPay: roundToDollar(employeeCut) };
}

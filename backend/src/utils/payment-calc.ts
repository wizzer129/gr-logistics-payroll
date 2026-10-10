export const EMPLOYEE_CUT_RATE = 0.42;

/** Round-half-up to the nearest whole dollar: .50+ rounds up, .49 and below rounds down. */
export function roundToDollar(amount: number): number {
	return Math.floor(amount + 0.5);
}

export function calculateEmployeeCut(summaryTotal: number): {
	employeeCut: number;
	roundedPay: number;
} {
	const employeeCut = summaryTotal * EMPLOYEE_CUT_RATE;
	return { employeeCut, roundedPay: roundToDollar(employeeCut) };
}

export function sumLoads(loads: { pay: number; extraPay?: number }[]): number {
	return loads.reduce((total, load) => total + load.pay + (load.extraPay ?? 0), 0);
}

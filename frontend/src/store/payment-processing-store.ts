import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { calculateEmployeeCut, DEFAULT_PAY_RATE_PERCENT } from '../utils/payment-calc';
import { getWeekRange, toIsoDate } from '../utils/week';
import { apiFetch } from '../utils/api';

export type Load = {
	storeName: string;
	loadDate: string;
	pay: number;
	extraPay?: number;
};

/** `loads` itemizes loads cut at the employee's rate; `lumpSum` is one amount paid in full. */
export type PaymentMode = 'loads' | 'lumpSum';

export const usePaymentProcessingStore = defineStore('payment-processing', () => {
	const employeeId = ref<string | null>(null);
	const payRatePercent = ref<number>(DEFAULT_PAY_RATE_PERCENT);
	const selectedDate = ref<string>(toIsoDate(new Date()));
	const payday = ref<string>(toIsoDate(new Date()));
	const loads = ref<Load[]>([]);
	const mode = ref<PaymentMode>('loads');
	const lumpSum = ref<number>(0);

	const isLumpSum = computed(() => mode.value === 'lumpSum');
	const weekRange = computed(() => getWeekRange(selectedDate.value));
	const summaryTotal = computed(() =>
		isLumpSum.value
			? lumpSum.value
			: loads.value.reduce((sum, l) => sum + l.pay + (l.extraPay ?? 0), 0),
	);
	// A lump sum is paid in full: no cut, and no rounding to the dollar (the backend matches).
	const effectiveRatePercent = computed(() => (isLumpSum.value ? 100 : payRatePercent.value));
	const cut = computed(() =>
		isLumpSum.value
			? { employeeCut: lumpSum.value, roundedPay: lumpSum.value }
			: calculateEmployeeCut(summaryTotal.value, payRatePercent.value),
	);
	const hasPayment = computed(() =>
		isLumpSum.value ? lumpSum.value > 0 : loads.value.length > 0,
	);

	function addLoad(load: Load): void {
		loads.value.push(load);
	}

	function replaceLoads(newLoads: Load[]): void {
		loads.value = newLoads;
	}

	function removeLoad(index: number): void {
		loads.value.splice(index, 1);
	}

	/** `complete` saves it as completed; otherwise it's saved as a draft. */
	async function submit({ complete }: { complete: boolean }): Promise<void> {
		if (!employeeId.value) throw new Error('No employee selected');

		const response = await apiFetch('/api/paycheck-logs', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({
				employeeId: employeeId.value,
				weekStart: weekRange.value.weekStart,
				weekEnd: weekRange.value.weekEnd,
				payday: payday.value,
				...(isLumpSum.value ? { lumpSum: lumpSum.value } : { loads: loads.value }),
				complete,
			}),
		});

		if (!response.ok) {
			const body = await response.json().catch(() => null);
			throw new Error(body?.error ?? `Failed to submit paycheck log (${response.status})`);
		}
	}

	function reset(): void {
		employeeId.value = null;
		payRatePercent.value = DEFAULT_PAY_RATE_PERCENT;
		selectedDate.value = toIsoDate(new Date());
		payday.value = toIsoDate(new Date());
		loads.value = [];
		mode.value = 'loads';
		lumpSum.value = 0;
	}

	return {
		employeeId,
		payRatePercent,
		selectedDate,
		payday,
		weekRange,
		loads,
		mode,
		lumpSum,
		isLumpSum,
		summaryTotal,
		effectiveRatePercent,
		cut,
		hasPayment,
		addLoad,
		replaceLoads,
		removeLoad,
		submit,
		reset,
	};
});

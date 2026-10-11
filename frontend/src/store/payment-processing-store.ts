import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { calculateEmployeeCut, DEFAULT_PAY_RATE_PERCENT } from '../utils/payment-calc';
import { getWeekRange, toIsoDate } from '../utils/week';

export type Load = {
	storeName: string;
	loadDate: string;
	pay: number;
	extraPay?: number;
};

export const usePaymentProcessingStore = defineStore('payment-processing', () => {
	const employeeId = ref<string | null>(null);
	const payRatePercent = ref<number>(DEFAULT_PAY_RATE_PERCENT);
	const selectedDate = ref<string>(toIsoDate(new Date()));
	const payday = ref<string>(toIsoDate(new Date()));
	const loads = ref<Load[]>([]);
	const completed = ref(false);

	const weekRange = computed(() => getWeekRange(selectedDate.value));
	const summaryTotal = computed(() =>
		loads.value.reduce((sum, l) => sum + l.pay + (l.extraPay ?? 0), 0),
	);
	const cut = computed(() => calculateEmployeeCut(summaryTotal.value, payRatePercent.value));

	function addLoad(load: Load): void {
		loads.value.push(load);
	}

	function removeLoad(index: number): void {
		loads.value.splice(index, 1);
	}

	async function submit(): Promise<void> {
		if (!employeeId.value) throw new Error('No employee selected');

		const response = await fetch('/api/paycheck-logs', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			credentials: 'include',
			body: JSON.stringify({
				employeeId: employeeId.value,
				weekStart: weekRange.value.weekStart,
				weekEnd: weekRange.value.weekEnd,
				payday: payday.value,
				loads: loads.value,
			}),
		});

		if (!response.ok) throw new Error('Failed to submit paycheck log');
		completed.value = true;
	}

	function reset(): void {
		employeeId.value = null;
		payRatePercent.value = DEFAULT_PAY_RATE_PERCENT;
		selectedDate.value = toIsoDate(new Date());
		payday.value = toIsoDate(new Date());
		loads.value = [];
		completed.value = false;
	}

	return {
		employeeId,
		payRatePercent,
		selectedDate,
		payday,
		weekRange,
		loads,
		completed,
		summaryTotal,
		cut,
		addLoad,
		removeLoad,
		submit,
		reset,
	};
});

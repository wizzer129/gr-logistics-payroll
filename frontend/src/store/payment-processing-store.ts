import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { calculateEmployeeCut } from '../utils/payment-calc';

export type Load = {
	storeName: string;
	loadDate: string;
	pay: number;
	extraPay: number;
};

export const usePaymentProcessingStore = defineStore('payment-processing', () => {
	const employeeId = ref<string | null>(null);
	const loads = ref<Load[]>([]);
	const completed = ref(false);

	const summaryTotal = computed(() =>
		loads.value.reduce((sum, l) => sum + l.pay + l.extraPay, 0),
	);
	const cut = computed(() => calculateEmployeeCut(summaryTotal.value));

	function addLoad(load: Load): void {
		loads.value.push(load);
	}

	function removeLoad(index: number): void {
		loads.value.splice(index, 1);
	}

	async function submit(): Promise<void> {
		if (!employeeId.value) throw new Error('No employee selected');

		const today = new Date();
		const weekStart = new Date(today);
		weekStart.setDate(today.getDate() - today.getDay());
		const weekEnd = new Date(weekStart);
		weekEnd.setDate(weekStart.getDate() + 6);

		const response = await fetch('/api/weekly-logs', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			credentials: 'include',
			body: JSON.stringify({
				employeeId: employeeId.value,
				weekStart: weekStart.toISOString().slice(0, 10),
				weekEnd: weekEnd.toISOString().slice(0, 10),
				loads: loads.value,
			}),
		});

		if (!response.ok) throw new Error('Failed to submit weekly log');
		completed.value = true;
	}

	function reset(): void {
		employeeId.value = null;
		loads.value = [];
		completed.value = false;
	}

	return { employeeId, loads, completed, summaryTotal, cut, addLoad, removeLoad, submit, reset };
});

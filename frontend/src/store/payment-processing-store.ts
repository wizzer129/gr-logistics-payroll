import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { calculateEmployeeCut, DEFAULT_PAY_RATE_PERCENT } from '../utils/payment-calc';
import { getWeekRange, toIsoDate } from '../utils/week';
import { apiJson, apiRequest } from '../utils/api';
import { i18n } from '../plugins/i18n';

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
	// Once saved as a draft, the form stays open on it: saving again or completing updates
	// that log instead of creating another. `savedState` is what was saved, for `isDirty`.
	const draftId = ref<string | null>(null);
	const savedState = ref<string | null>(null);

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

	/** What a save sends, minus `complete`. The employee is only sent on create. */
	const payload = computed(() => ({
		weekStart: weekRange.value.weekStart,
		weekEnd: weekRange.value.weekEnd,
		payday: payday.value,
		...(isLumpSum.value ? { lumpSum: lumpSum.value } : { loads: loads.value }),
	}));

	/** Anything entered that isn't saved: since the last draft save, or at all. */
	const isDirty = computed(() =>
		draftId.value
			? JSON.stringify(payload.value) !== savedState.value
			: loads.value.length > 0 || lumpSum.value > 0,
	);

	/**
	 * `complete` saves it as completed; otherwise it's saved as a draft and stays open. An
	 * open draft is updated in place (its employee can't change; see the view).
	 */
	async function submit({ complete }: { complete: boolean }): Promise<void> {
		if (!employeeId.value) throw new Error(i18n.global.t('payment.noEmployee'));
		const body = { ...payload.value, complete };

		if (draftId.value) {
			await apiRequest(`/api/paycheck-logs/${draftId.value}`, {
				method: 'PUT',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify(body),
			});
		} else {
			const created = await apiJson<{ id: string }>('/api/paycheck-logs', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ employeeId: employeeId.value, ...body }),
			});
			draftId.value = created.id;
		}
		savedState.value = JSON.stringify(payload.value);
	}

	function reset(): void {
		employeeId.value = null;
		payRatePercent.value = DEFAULT_PAY_RATE_PERCENT;
		selectedDate.value = toIsoDate(new Date());
		payday.value = toIsoDate(new Date());
		loads.value = [];
		mode.value = 'loads';
		lumpSum.value = 0;
		draftId.value = null;
		savedState.value = null;
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
		draftId,
		isDirty,
		addLoad,
		replaceLoads,
		removeLoad,
		submit,
		reset,
	};
});

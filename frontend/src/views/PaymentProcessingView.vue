<script setup lang="ts">
import { computed, onMounted, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { usePaymentProcessingStore, type Load } from '../store/payment-processing-store';
import { parseLocalDate } from '../utils/week';
import { useEmployees, type EmployeeOption } from '../composables/useEmployees';
import LoadForm from '../components/payment-processing/LoadForm.vue';
import LoadList from '../components/payment-processing/LoadList.vue';
import PaymentSummary from '../components/payment-processing/PaymentSummary.vue';

const { t, locale } = useI18n();
const store = usePaymentProcessingStore();
const { employees, loadEmployees } = useEmployees();

function employeeDisplayName(employee: EmployeeOption): string {
	return `${employee.firstName} ${employee.lastName}`;
}

onMounted(loadEmployees);

// Keep the store's rate in sync with whichever employee is selected — each can be cut at a
// different percentage (see employees.pay_rate_percent).
watch(
	() => store.employeeId,
	(id) => {
		const employee = employees.value.find((e) => e.id === id);
		if (employee) store.payRatePercent = employee.payRatePercent;
	},
);

const weekRangeLabel = computed(() => {
	const formatter = new Intl.DateTimeFormat(locale.value, { month: 'short', day: 'numeric' });
	return t('payment.weekRange', {
		start: formatter.format(parseLocalDate(store.weekRange.weekStart)),
		end: formatter.format(parseLocalDate(store.weekRange.weekEnd)),
	});
});

function onAddLoad(load: Load): void {
	store.addLoad(load);
}

function onRemoveLoad(index: number): void {
	store.removeLoad(index);
}

async function onExtractRequested(file: File): Promise<void> {
	const formData = new FormData();
	formData.append('screenshot', file);

	const response = await fetch('/api/loads/extract', {
		method: 'POST',
		credentials: 'include',
		body: formData,
	});

	if (!response.ok) return;

	const extracted = await response.json();
	store.addLoad({
		storeName: extracted.storeName,
		loadDate: extracted.date,
		pay: extracted.payment,
		extraPay: extracted.extraPay ?? 0,
	});
}

async function onComplete(): Promise<void> {
	await store.submit();
}

function onPrint(): void {
	window.print();
}
</script>

<template>
	<v-container>
		<v-row>
			<v-col cols="12" sm="6" md="4" lg="2">
				<v-select
					v-model="store.employeeId"
					:items="employees"
					:item-title="employeeDisplayName"
					item-value="id"
					:label="t('dashboard.selectEmployee')"
					variant="solo-filled"
				/>
			</v-col>
			<v-col cols="12" sm="6" md="4" lg="2">
				<v-text-field
					v-model="store.selectedDate"
					:label="t('payment.weekOf')"
					type="date"
					variant="solo-filled"
				/>
				<div class="text-caption text-medium-emphasis mt-n4 mb-2">{{ weekRangeLabel }}</div>
			</v-col>
			<v-col cols="12" sm="6" md="4" lg="2">
				<v-text-field
					v-model="store.payday"
					:label="t('payment.payday')"
					type="date"
					variant="solo-filled"
				/>
			</v-col>
		</v-row>

		<v-row>
			<v-col cols="12" md="6">
				<LoadForm @add-load="onAddLoad" @extract-requested="onExtractRequested" />
			</v-col>
			<v-col cols="12" md="6">
				<LoadList :loads="store.loads" @remove="onRemoveLoad" />
				<PaymentSummary
					class="mt-4"
					:summary-total="store.summaryTotal"
					:employee-cut="store.cut.employeeCut"
					:rounded-pay="store.cut.roundedPay"
					:pay-rate-percent="store.payRatePercent"
					:completed="store.completed"
					@complete="onComplete"
					@print="onPrint"
				/>
			</v-col>
		</v-row>

		<!-- Print layout: fits the same table twice, stacked -->
		<div class="print-only">
			<LoadList :loads="store.loads" @remove="() => {}" />
			<LoadList :loads="store.loads" @remove="() => {}" />
		</div>
	</v-container>
</template>

<style scoped>
.print-only {
	display: none;
}

@media print {
	.print-only {
		display: block;
	}
}
</style>

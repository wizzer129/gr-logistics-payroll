<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { usePaymentProcessingStore, type Load } from '../store/payment-processing-store';
import { parseLocalDate } from '../utils/week';
import LoadForm from '../components/payment-processing/LoadForm.vue';
import LoadList from '../components/payment-processing/LoadList.vue';
import PaymentSummary from '../components/payment-processing/PaymentSummary.vue';

const { t, locale } = useI18n();
const store = usePaymentProcessingStore();

const employees = ref<{ id: string; firstName: string; lastName: string }[]>([]);

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
			<v-col cols="12" md="6">
				<v-select
					v-model="store.employeeId"
					:items="employees"
					item-title="lastName"
					item-value="id"
					:label="t('dashboard.selectEmployee')"
				/>
			</v-col>
			<v-col cols="12" md="6">
				<v-text-field
					v-model="store.selectedDate"
					:label="t('payment.weekOf')"
					type="date"
				/>
				<div class="text-caption text-medium-emphasis mt-n4 mb-2">{{ weekRangeLabel }}</div>
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

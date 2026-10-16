<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useDisplay } from 'vuetify';
import { usePaymentProcessingStore, type Load } from '../store/payment-processing-store';
import { useToastStore } from '../store/toast-store';
import { parseLocalDate } from '../utils/week';
import { useEmployees, type EmployeeOption } from '../composables/useEmployees';
import { matchEmployeeByName, type ParsedLoadsCsv } from '../utils/csv-loads';
import LoadEntryCard from '../components/payment-processing/LoadEntryCard.vue';
import LoadList from '../components/payment-processing/LoadList.vue';
import PaymentSummary from '../components/payment-processing/PaymentSummary.vue';
import PaymentHistory from '../components/payment-processing/PaymentHistory.vue';
import PaystubSheet from '../components/payment-processing/PaystubSheet.vue';
import PaystubPrinter from '../components/payment-processing/PaystubPrinter.vue';
import type { PaystubData } from '../components/payment-processing/Paystub.vue';

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

const csvEmployeeNotFound = ref<string | null>(null);

// A CSV is one employee's whole week: it replaces the current loads, sets the employee,
// week and payday from the file. Totals and the cut are computed here, not read from it.
function onCsvProcessed(csv: ParsedLoadsCsv): void {
	const employee = matchEmployeeByName(csv.employeeName, employees.value);
	csvEmployeeNotFound.value = employee ? null : csv.employeeName;
	if (employee) store.employeeId = employee.id;

	store.selectedDate = csv.loads[0].loadDate;
	store.payday = csv.payday;
	store.replaceLoads(csv.loads);
}

// Removing the CSV takes its loads back out. Employee, week and payday are left as they are.
function onCsvCleared(): void {
	csvEmployeeNotFound.value = null;
	store.replaceLoads([]);
}

// Which button is saving, so only that one shows a spinner.
const processing = ref<'draft' | 'complete' | null>(null);
const toast = useToastStore();
// Bumped to remount the entry card, which drops the uploaded CSV and the manual form.
const entryCardKey = ref(0);
const history = ref<InstanceType<typeof PaymentHistory> | null>(null);
// The history gutter is desktop-only; below lg there isn't room beside the two columns.
const { lgAndUp } = useDisplay();

const HISTORY_COLLAPSED_KEY = 'gr-logistics-history-collapsed';
const historyCollapsed = ref(localStorage.getItem(HISTORY_COLLAPSED_KEY) === 'true');
watch(historyCollapsed, (collapsed) =>
	localStorage.setItem(HISTORY_COLLAPSED_KEY, String(collapsed)),
);

const canComplete = computed(() => Boolean(store.employeeId) && store.hasPayment);

async function onSubmit(complete: boolean): Promise<void> {
	processing.value = complete ? 'complete' : 'draft';
	try {
		await store.submit({ complete });
	} catch (e) {
		toast.error(t('payment.processFailed', { error: (e as Error).message }));
		return;
	} finally {
		processing.value = null;
	}

	toast.success(t(complete ? 'payment.processed' : 'payment.draftSaved'));
	store.reset();
	csvEmployeeNotFound.value = null;
	entryCardKey.value++;
	await history.value?.refresh();
}

const paystubOpen = ref(false);

const selectedEmployeeName = computed(() => {
	const employee = employees.value.find((e) => e.id === store.employeeId);
	return employee ? employeeDisplayName(employee) : '';
});

const printer = ref<InstanceType<typeof PaystubPrinter> | null>(null);

function onPrintPaystub(): void {
	printer.value?.print(paystubData.value);
}

const paystubData = computed<PaystubData>(() => ({
	employeeName: selectedEmployeeName.value || t('paystub.noEmployee'),
	weekStart: store.weekRange.weekStart,
	weekEnd: store.weekRange.weekEnd,
	payday: store.payday,
	loads: store.loads,
	summaryTotal: store.summaryTotal,
	employeeCut: store.cut.employeeCut,
	roundedPay: store.cut.roundedPay,
	payRatePercent: store.effectiveRatePercent,
	lumpSum: store.isLumpSum,
}));
</script>

<template>
	<v-container>
		<v-row>
			<v-col cols="12" sm="6" md="4" xl="3">
				<v-select
					v-model="store.employeeId"
					:items="employees"
					:item-title="employeeDisplayName"
					item-value="id"
					:label="t('dashboard.selectEmployee')"
					variant="solo-filled"
				/>
			</v-col>
			<v-col cols="12" sm="6" md="4" xl="3">
				<v-text-field
					v-model="store.selectedDate"
					:label="t('payment.weekOf')"
					type="date"
					variant="solo-filled"
				/>
				<div class="text-caption text-medium-emphasis mt-n4 mb-2">{{ weekRangeLabel }}</div>
			</v-col>
			<v-col cols="12" sm="6" md="4" xl="3">
				<v-text-field
					v-model="store.payday"
					:label="t('payment.payday')"
					type="date"
					variant="solo-filled"
				/>
			</v-col>
		</v-row>

		<v-alert
			v-if="csvEmployeeNotFound"
			type="warning"
			density="compact"
			class="mb-4"
			closable
			@click:close="csvEmployeeNotFound = null"
		>
			{{ t('payment.csvEmployeeNotFound', { name: csvEmployeeNotFound }) }}
		</v-alert>

		<v-row>
			<v-col cols="12" md="6">
				<LoadEntryCard
					:key="entryCardKey"
					v-model:mode="store.mode"
					v-model:lump-sum="store.lumpSum"
					:pay-rate-percent="store.payRatePercent"
					@add-load="onAddLoad"
					@csv-processed="onCsvProcessed"
					@csv-cleared="onCsvCleared"
				/>
			</v-col>
			<v-col cols="12" md="6">
				<LoadList v-if="!store.isLumpSum" :loads="store.loads" @remove="onRemoveLoad" />
				<PaymentSummary
					class="mt-4"
					:summary-total="store.summaryTotal"
					:employee-cut="store.cut.employeeCut"
					:rounded-pay="store.cut.roundedPay"
					:pay-rate-percent="store.effectiveRatePercent"
					:has-payment="store.hasPayment"
					:can-complete="canComplete"
					:processing="processing"
					@complete="onSubmit(true)"
					@save-draft="onSubmit(false)"
					@paystub="paystubOpen = true"
				/>
			</v-col>
		</v-row>

		<v-dialog v-model="paystubOpen" width="auto" scrollable>
			<v-card>
				<v-card-text class="paystub-preview">
					<PaystubSheet :stub="paystubData" />
				</v-card-text>
				<v-card-actions>
					<v-spacer />
					<v-btn variant="outlined" prepend-icon="mdi-printer" @click="onPrintPaystub">{{
						t('payment.print')
					}}</v-btn>
					<v-btn variant="outlined" @click="paystubOpen = false">{{
						t('common.close')
					}}</v-btn>
				</v-card-actions>
			</v-card>
		</v-dialog>

		<PaystubPrinter ref="printer" />

		<!-- A right-hand gutter: full height, fixed to the window while the page scrolls. -->
		<template v-if="lgAndUp">
			<v-navigation-drawer v-if="!historyCollapsed" location="right" permanent width="400">
				<PaymentHistory
					ref="history"
					class="fill-height"
					@collapse="historyCollapsed = true"
				/>
			</v-navigation-drawer>
			<!-- Collapsed, the drawer is gone entirely; a tab on the right edge brings it back. -->
			<v-btn
				v-else
				class="history-expand"
				icon="mdi-chevron-left"
				:aria-label="t('history.expand')"
				:title="t('history.expand')"
				elevation="4"
				@click="historyCollapsed = false"
			/>
		</template>
	</v-container>
</template>

<style scoped>
/* Below the 64px app bar, flush against the right edge of the window. */
.history-expand {
	position: fixed;
	bottom: 80px;
	right: 0;
	z-index: 1004;
	border-radius: 6px 0px 0px 6px;
}

/* A neutral "desk" around the paper so the stub reads as a document in either theme. */
.paystub-preview {
	padding: 24px !important;
	background: rgba(var(--v-theme-on-surface), 0.06);
}

.paystub-preview :deep(.paystub-sheet .paystub) {
	box-shadow: 0 2px 10px rgba(0, 0, 0, 0.15);
}
</style>

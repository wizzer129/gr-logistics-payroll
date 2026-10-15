<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useEmployees, type EmployeeOption } from '../composables/useEmployees';
import { calculateEmployeeCut, DEFAULT_PAY_RATE_PERCENT } from '../utils/payment-calc';
import { formatCurrency } from '../utils/currency';
import { formatMonthDay } from '../utils/week';
import type { Load } from '../store/payment-processing-store';
import LoadForm from '../components/payment-processing/LoadForm.vue';
import LoadList from '../components/payment-processing/LoadList.vue';
import PaystubPrinter from '../components/payment-processing/PaystubPrinter.vue';
import type { PaystubData } from '../components/payment-processing/Paystub.vue';
import { apiFetch } from '../utils/api';

type PaycheckLogRow = {
	id: string;
	employeeId: string;
	weekStart: string;
	weekEnd: string;
	payday: string;
	status: string;
	/** `loads` or `lump_sum` (no loads, paid in full). */
	kind: 'loads' | 'lump_sum';
	summaryTotal: number;
	employeeCut: number;
	// The rate this log's cut was computed at; the backend recomputes edits at it too.
	payRatePercent: number;
	firstName: string;
	lastName: string;
};

const { t, locale } = useI18n();
const { employees, loadEmployees } = useEmployees();

const employeeId = ref<string | null>(null);
const from = ref('');
const to = ref('');
const results = ref<PaycheckLogRow[]>([]);

// --- Pagination (client-side: the list endpoint returns every matching log) ---

const PAGE_SIZE_OPTIONS = [5, 10, 25, 50];
const pageSize = ref(10);
const page = ref(1);

const pageCount = computed(() => Math.max(1, Math.ceil(results.value.length / pageSize.value)));
const pagedResults = computed(() =>
	results.value.slice((page.value - 1) * pageSize.value, page.value * pageSize.value),
);
const rangeLabel = computed(() => {
	const total = results.value.length;
	if (total === 0) return t('pagination.range', { start: 0, end: 0, total });
	const start = (page.value - 1) * pageSize.value + 1;
	return t('pagination.range', {
		start,
		end: Math.min(page.value * pageSize.value, total),
		total,
	});
});

watch(pageSize, () => {
	page.value = 1;
});
// Deleting the last row on the last page would leave it empty; step back a page.
watch(pageCount, (count) => {
	if (page.value > count) page.value = count;
});

function employeeDisplayName(employee: EmployeeOption): string {
	return `${employee.firstName} ${employee.lastName}`;
}

async function search(): Promise<void> {
	const params = new URLSearchParams();
	if (employeeId.value) params.set('employeeId', employeeId.value);
	if (from.value) params.set('from', from.value);
	if (to.value) params.set('to', to.value);

	// Backend already orders week_start DESC — latest to oldest by default.
	const response = await apiFetch(`/api/paycheck-logs?${params}`);
	if (response.ok) {
		results.value = await response.json();
		page.value = 1;
	}
}

// The employee dropdown is the primary filter — apply it immediately rather than waiting
// for the Search button, which stays for the secondary date-range filters.
watch(employeeId, search);

onMounted(() => {
	loadEmployees();
	search();
});

// --- Edit dialog ---

const editDialog = ref(false);
const editingLog = ref<PaycheckLogRow | null>(null);
const editLoads = ref<Load[]>([]);
const editForm = reactive({ weekStart: '', weekEnd: '', payday: '' });

const editRatePercent = ref(DEFAULT_PAY_RATE_PERCENT);
const editSummaryTotal = ref(0);
const editCut = ref(calculateEmployeeCut(0));
// A lump-sum log has no loads; its one amount is paid in full (no cut, no rounding).
const editIsLumpSum = ref(false);
const editLumpSum = ref(0);

function recalcEdit(): void {
	if (editIsLumpSum.value) {
		editSummaryTotal.value = editLumpSum.value;
		editCut.value = { employeeCut: editLumpSum.value, roundedPay: editLumpSum.value };
		return;
	}
	editSummaryTotal.value = editLoads.value.reduce((sum, l) => sum + l.pay + (l.extraPay ?? 0), 0);
	editCut.value = calculateEmployeeCut(editSummaryTotal.value, editRatePercent.value);
}

watch([editLoads, editLumpSum], recalcEdit, { deep: true });

async function openEdit(log: PaycheckLogRow): Promise<void> {
	editingLog.value = log;
	editError.value = null;
	// The API sends "YYYY-MM-DD"; <input type="date"> rejects anything with a time component,
	// so truncate defensively.
	editForm.weekStart = log.weekStart.slice(0, 10);
	editForm.weekEnd = log.weekEnd.slice(0, 10);
	editForm.payday = log.payday.slice(0, 10);
	// Not the employee's current rate: a rate change must not re-rate an existing log.
	editRatePercent.value = log.payRatePercent;
	editIsLumpSum.value = log.kind === 'lump_sum';
	editLumpSum.value = editIsLumpSum.value ? log.summaryTotal : 0;

	if (editIsLumpSum.value) {
		editLoads.value = [];
	} else {
		const response = await apiFetch(`/api/paycheck-logs/${log.id}/loads`);
		editLoads.value = response.ok ? await response.json() : [];
	}
	recalcEdit();
	editDialog.value = true;
}

function onEditAddLoad(load: Load): void {
	editLoads.value.push(load);
}

function onEditRemoveLoad(index: number): void {
	editLoads.value.splice(index, 1);
}

const editError = ref<string | null>(null);

/** `complete` also marks a draft completed, in the same request as the edit. */
async function saveEdit(complete = false): Promise<void> {
	if (!editingLog.value) return;
	editError.value = null;

	const response = await apiFetch(`/api/paycheck-logs/${editingLog.value.id}`, {
		method: 'PUT',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({
			weekStart: editForm.weekStart,
			weekEnd: editForm.weekEnd,
			payday: editForm.payday,
			...(editIsLumpSum.value ? { lumpSum: editLumpSum.value } : { loads: editLoads.value }),
			complete,
		}),
	});

	if (!response.ok) {
		const body = await response.json().catch(() => null);
		editError.value = body?.error ?? t('payment.saveFailed');
		return;
	}
	editDialog.value = false;
	await search();
}

// Prints what's in the dialog right now, saved or not.
const printer = ref<InstanceType<typeof PaystubPrinter> | null>(null);

const editPaystub = computed<PaystubData | null>(() => {
	const log = editingLog.value;
	if (!log) return null;
	return {
		employeeName: `${log.firstName} ${log.lastName}`,
		weekStart: editForm.weekStart,
		weekEnd: editForm.weekEnd,
		payday: editForm.payday,
		loads: editLoads.value,
		summaryTotal: editSummaryTotal.value,
		employeeCut: editCut.value.employeeCut,
		roundedPay: editCut.value.roundedPay,
		payRatePercent: editIsLumpSum.value ? 100 : editRatePercent.value,
		lumpSum: editIsLumpSum.value,
	};
});

function onPrintPaystub(): void {
	if (editPaystub.value) printer.value?.print(editPaystub.value);
}

function cancelEdit(): void {
	editDialog.value = false;
	editingLog.value = null;
}

async function onDelete(log: PaycheckLogRow): Promise<void> {
	if (!confirm(t('payment.confirmDelete'))) return;

	const response = await apiFetch(`/api/paycheck-logs/${log.id}`, {
		method: 'DELETE',
	});

	if (response.ok) results.value = results.value.filter((r) => r.id !== log.id);
}
</script>

<template>
	<v-container>
		<h1 class="text-h5 mb-4">{{ t('nav.editPayment') }}</h1>

		<v-row>
			<v-col cols="12" md="4">
				<v-select
					v-model="employeeId"
					:items="employees"
					:item-title="employeeDisplayName"
					item-value="id"
					:label="t('dashboard.selectEmployee')"
					:placeholder="t('common.allEmployees')"
					clearable
				/>
			</v-col>
			<v-col cols="12" md="3">
				<v-text-field v-model="from" label="From" type="date" />
			</v-col>
			<v-col cols="12" md="3">
				<v-text-field v-model="to" label="To" type="date" />
			</v-col>
			<v-col cols="12" md="2" class="d-flex align-center">
				<v-btn color="primary" @click="search">Search</v-btn>
			</v-col>
		</v-row>

		<v-table density="compact" striped="odd">
			<thead>
				<tr>
					<th>{{ t('employees.firstName') }} {{ t('employees.lastName') }}</th>
					<th>Week Start</th>
					<th>Week End</th>
					<th>{{ t('payment.payday') }}</th>
					<th>{{ t('payment.status') }}</th>
					<th class="text-right">{{ t('payment.totalPay') }}</th>
					<th>{{ t('common.actions') }}</th>
				</tr>
			</thead>
			<tbody>
				<tr v-for="log in pagedResults" :key="log.id">
					<td>{{ log.firstName }} {{ log.lastName }}</td>
					<td>{{ formatMonthDay(log.weekStart, locale) }}</td>
					<td>{{ formatMonthDay(log.weekEnd, locale) }}</td>
					<td>{{ formatMonthDay(log.payday, locale) }}</td>
					<td>
						{{ log.status }}
						<v-chip
							v-if="log.kind === 'lump_sum'"
							size="x-small"
							variant="outlined"
							color="primary"
							class="ml-1"
							>{{ t('payment.lumpSum') }}</v-chip
						>
					</td>
					<td class="text-right">{{ formatCurrency(log.employeeCut) }}</td>
					<td>
						<v-btn
							icon="mdi-pencil"
							size="small"
							variant="text"
							:aria-label="t('common.edit')"
							@click="openEdit(log)"
						/>
						<v-btn
							icon="mdi-delete"
							size="small"
							variant="text"
							:aria-label="t('common.delete')"
							@click="onDelete(log)"
						/>
					</td>
				</tr>
			</tbody>
		</v-table>

		<div class="d-flex flex-wrap align-center ga-4 mt-2">
			<v-select
				v-model="pageSize"
				:items="PAGE_SIZE_OPTIONS"
				:label="t('pagination.pageSize')"
				density="compact"
				variant="outlined"
				hide-details
				class="page-size"
			/>
			<span class="text-body-2 text-medium-emphasis">{{ rangeLabel }}</span>
			<v-spacer />
			<v-pagination
				v-model="page"
				:length="pageCount"
				:total-visible="5"
				density="comfortable"
				rounded
			/>
		</div>

		<v-dialog v-model="editDialog" max-width="900">
			<v-card :title="t('payment.editTitle')">
				<v-card-text>
					<v-alert v-if="editError" type="error" density="compact" class="mb-4">{{
						editError
					}}</v-alert>
					<v-row>
						<v-col cols="12" md="4">
							<v-text-field
								v-model="editForm.weekStart"
								label="Week Start"
								type="date"
							/>
						</v-col>
						<v-col cols="12" md="4">
							<v-text-field v-model="editForm.weekEnd" label="Week End" type="date" />
						</v-col>
						<v-col cols="12" md="4">
							<v-text-field
								v-model="editForm.payday"
								:label="t('payment.payday')"
								type="date"
							/>
						</v-col>
					</v-row>

					<v-row>
						<v-col v-if="editIsLumpSum" cols="12" md="6">
							<v-text-field
								v-model.number="editLumpSum"
								:label="t('payment.lumpSumAmount')"
								:hint="t('payment.lumpSumHint')"
								persistent-hint
								type="number"
								prefix="$"
								min="0"
								step="0.01"
							/>
						</v-col>
						<v-col v-else cols="12" md="6">
							<v-card :title="t('payment.addManualLoad')">
								<v-card-text>
									<LoadForm button-variant="outlined" @add-load="onEditAddLoad" />
								</v-card-text>
							</v-card>
						</v-col>
						<v-col cols="12" md="6">
							<LoadList
								v-if="!editIsLumpSum"
								:loads="editLoads"
								@remove="onEditRemoveLoad"
							/>

							<v-card :class="{ 'mt-4': !editIsLumpSum }">
								<v-card-text>
									<div class="d-flex justify-space-between">
										<span>{{ t('payment.summaryTotal') }}</span
										><strong>{{ formatCurrency(editSummaryTotal) }}</strong>
									</div>
									<div class="d-flex justify-space-between">
										<span>{{
											t('payment.employeeCut', { rate: editRatePercent })
										}}</span
										><strong>{{ formatCurrency(editCut.employeeCut) }}</strong>
									</div>
									<div class="d-flex justify-space-between">
										<span>{{ t('payment.totalPay') }}</span
										><strong>{{ formatCurrency(editCut.roundedPay) }}</strong>
									</div>
								</v-card-text>
							</v-card>
						</v-col>
					</v-row>
				</v-card-text>
				<v-card-actions>
					<v-btn variant="outlined" prepend-icon="mdi-printer" @click="onPrintPaystub">{{
						t('payment.printPaystub')
					}}</v-btn>
					<v-spacer />
					<v-btn variant="outlined" @click="cancelEdit">{{ t('common.cancel') }}</v-btn>
					<v-btn variant="outlined" color="primary" @click="saveEdit()">{{
						t('common.save')
					}}</v-btn>
					<!-- Only drafts can be completed; a completed log just saves. -->
					<v-btn
						v-if="editingLog?.status === 'draft'"
						variant="outlined"
						color="success"
						@click="saveEdit(true)"
						>{{ t('payment.complete') }}</v-btn
					>
				</v-card-actions>
			</v-card>
		</v-dialog>

		<PaystubPrinter ref="printer" />
	</v-container>
</template>

<style scoped>
.page-size {
	max-width: 8rem;
}
</style>

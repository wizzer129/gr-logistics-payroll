<script setup lang="ts">
import { onMounted, reactive, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useEmployees, type EmployeeOption } from '../composables/useEmployees';
import { calculateEmployeeCut, DEFAULT_PAY_RATE_PERCENT } from '../utils/payment-calc';
import { formatCurrency } from '../utils/currency';
import { formatMonthDay } from '../utils/week';
import type { Load } from '../store/payment-processing-store';
import LoadForm from '../components/payment-processing/LoadForm.vue';
import LoadList from '../components/payment-processing/LoadList.vue';

type PaycheckLogRow = {
	id: string;
	employeeId: string;
	weekStart: string;
	weekEnd: string;
	payday: string;
	status: string;
	summaryTotal: number;
	employeeCut: number;
	firstName: string;
	lastName: string;
};

const { t, locale } = useI18n();
const { employees, loadEmployees } = useEmployees();

const employeeId = ref<string | null>(null);
const from = ref('');
const to = ref('');
const results = ref<PaycheckLogRow[]>([]);

function employeeDisplayName(employee: EmployeeOption): string {
	return `${employee.firstName} ${employee.lastName}`;
}

async function search(): Promise<void> {
	const params = new URLSearchParams();
	if (employeeId.value) params.set('employeeId', employeeId.value);
	if (from.value) params.set('from', from.value);
	if (to.value) params.set('to', to.value);

	// Backend already orders week_start DESC — latest to oldest by default.
	const response = await fetch(`/api/paycheck-logs?${params}`, { credentials: 'include' });
	if (response.ok) results.value = await response.json();
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

function recalcEdit(): void {
	editSummaryTotal.value = editLoads.value.reduce(
		(sum, l) => sum + l.pay + (l.extraPay ?? 0),
		0,
	);
	editCut.value = calculateEmployeeCut(editSummaryTotal.value, editRatePercent.value);
}

watch(editLoads, recalcEdit, { deep: true });

async function openEdit(log: PaycheckLogRow): Promise<void> {
	editingLog.value = log;
	// API dates can come back as plain "YYYY-MM-DD" or full ISO datetimes — <input type="date">
	// rejects anything with a time component, so always truncate to the date-only prefix.
	editForm.weekStart = log.weekStart.slice(0, 10);
	editForm.weekEnd = log.weekEnd.slice(0, 10);
	editForm.payday = log.payday.slice(0, 10);
	editRatePercent.value =
		employees.value.find((e) => e.id === log.employeeId)?.payRatePercent ??
		DEFAULT_PAY_RATE_PERCENT;

	const response = await fetch(`/api/paycheck-logs/${log.id}/loads`, { credentials: 'include' });
	editLoads.value = response.ok ? await response.json() : [];
	recalcEdit();
	editDialog.value = true;
}

function onEditAddLoad(load: Load): void {
	editLoads.value.push(load);
}

function onEditRemoveLoad(index: number): void {
	editLoads.value.splice(index, 1);
}

async function onEditExtractRequested(file: File): Promise<void> {
	const formData = new FormData();
	formData.append('screenshot', file);

	const response = await fetch('/api/loads/extract', {
		method: 'POST',
		credentials: 'include',
		body: formData,
	});

	if (!response.ok) return;

	const extracted = await response.json();
	onEditAddLoad({
		storeName: extracted.storeName,
		loadDate: extracted.date,
		pay: extracted.payment,
		extraPay: extracted.extraPay ?? 0,
	});
}

async function saveEdit(): Promise<void> {
	if (!editingLog.value) return;

	const response = await fetch(`/api/paycheck-logs/${editingLog.value.id}`, {
		method: 'PUT',
		headers: { 'Content-Type': 'application/json' },
		credentials: 'include',
		body: JSON.stringify({
			weekStart: editForm.weekStart,
			weekEnd: editForm.weekEnd,
			payday: editForm.payday,
			loads: editLoads.value,
		}),
	});

	if (!response.ok) return;
	editDialog.value = false;
	await search();
}

function cancelEdit(): void {
	editDialog.value = false;
	editingLog.value = null;
}

async function onDelete(log: PaycheckLogRow): Promise<void> {
	if (!confirm(t('payment.confirmDelete'))) return;

	const response = await fetch(`/api/paycheck-logs/${log.id}`, {
		method: 'DELETE',
		credentials: 'include',
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
				<tr v-for="log in results" :key="log.id">
					<td>{{ log.firstName }} {{ log.lastName }}</td>
					<td>{{ formatMonthDay(log.weekStart, locale) }}</td>
					<td>{{ formatMonthDay(log.weekEnd, locale) }}</td>
					<td>{{ formatMonthDay(log.payday, locale) }}</td>
					<td>{{ log.status }}</td>
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

		<v-dialog v-model="editDialog" max-width="900">
			<v-card :title="t('payment.editTitle')">
				<v-card-text>
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
						<v-col cols="12" md="6">
							<LoadForm @add-load="onEditAddLoad" @extract-requested="onEditExtractRequested" />
						</v-col>
						<v-col cols="12" md="6">
							<LoadList :loads="editLoads" @remove="onEditRemoveLoad" />

							<v-card class="mt-4">
								<v-card-text>
									<div class="d-flex justify-space-between">
										<span>{{ t('payment.summaryTotal') }}</span
										><strong>{{ formatCurrency(editSummaryTotal) }}</strong>
									</div>
									<div class="d-flex justify-space-between">
										<span>{{ t('payment.employeeCut', { rate: editRatePercent }) }}</span
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
					<v-spacer />
					<v-btn @click="cancelEdit">{{ t('common.cancel') }}</v-btn>
					<v-btn color="primary" @click="saveEdit">{{ t('common.save') }}</v-btn>
				</v-card-actions>
			</v-card>
		</v-dialog>
	</v-container>
</template>

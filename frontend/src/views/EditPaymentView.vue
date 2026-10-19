<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useDisplay } from 'vuetify';
import { useEmployees, type EmployeeOption } from '../composables/useEmployees';
import { calculateEmployeeCut, DEFAULT_PAY_RATE_PERCENT } from '../utils/payment-calc';
import { formatCurrency } from '../utils/currency';
import { formatMonthDay } from '../utils/week';
import type { Load } from '../store/payment-processing-store';
import LoadForm from '../components/payment-processing/LoadForm.vue';
import LoadList from '../components/payment-processing/LoadList.vue';
import PaystubPrinter from '../components/payment-processing/PaystubPrinter.vue';
import PaystubPreviewDialog from '../components/payment-processing/PaystubPreviewDialog.vue';
import EmailPaystubDialog from '../components/email/EmailPaystubDialog.vue';
import type { PaystubData } from '../components/payment-processing/Paystub.vue';
import { useConfirmStore } from '../store/confirm-store';
import { useUnsavedChanges } from '../composables/useUnsavedChanges';
import { useToastStore } from '../store/toast-store';
import { ApiError, apiJson, apiRequest } from '../utils/api';

type PaycheckLogRow = {
	id: string;
	employeeId: string;
	weekStart: string;
	weekEnd: string;
	payday: string;
	status: 'draft' | 'completed';
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
// Phones get stacked list rows instead of the 7-column table, and a fullscreen edit dialog.
const { xs } = useDisplay();
const toast = useToastStore();
const confirmDialog = useConfirmStore();
const { confirmDiscard, guardLeave } = useUnsavedChanges();
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
	try {
		results.value = await apiJson<PaycheckLogRow[]>(`/api/paycheck-logs?${params}`);
		page.value = 1;
	} catch (error) {
		toast.fromError(error);
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
		// Without its loads the dialog would show (and could save) an empty paycheck.
		try {
			editLoads.value = await apiJson<Load[]>(`/api/paycheck-logs/${log.id}/loads`);
		} catch (error) {
			toast.fromError(error);
			return;
		}
	}
	recalcEdit();
	editSnapshot.value = editState();
	editDialog.value = true;
}

// What the dialog edits, serialized: dirty means it differs from when the dialog opened.
const editSnapshot = ref('');
function editState(): string {
	return JSON.stringify([editForm, editLoads.value, editLumpSum.value]);
}
const editLoadForm = ref<InstanceType<typeof LoadForm> | null>(null);
const editIsDirty = computed(
	() =>
		editDialog.value &&
		(editState() !== editSnapshot.value || Boolean(editLoadForm.value?.isDirty)),
);

guardLeave(() => editIsDirty.value);

// Cancel, Escape and clicking outside all come through here.
async function requestCloseEdit(): Promise<void> {
	if (await confirmDiscard(() => editIsDirty.value)) cancelEdit();
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

	try {
		await apiRequest(`/api/paycheck-logs/${editingLog.value.id}`, {
			method: 'PUT',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({
				weekStart: editForm.weekStart,
				weekEnd: editForm.weekEnd,
				payday: editForm.payday,
				...(editIsLumpSum.value
					? { lumpSum: editLumpSum.value }
					: { loads: editLoads.value }),
				complete,
			}),
		});
	} catch (error) {
		// Shown in the dialog, which stays open so the edit isn't lost.
		if (!(error instanceof ApiError)) throw error;
		editError.value = error.message;
		return;
	}
	editDialog.value = false;
	toast.success(t(complete ? 'payment.completed' : 'payment.updated'));
	await search();
}

// Previews and prints what's in the dialog right now, saved or not.
const paystubOpen = ref(false);
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

// The email always sends the saved log, so it's only offered for completed logs, and
// in the edit dialog only once its changes are saved.
const emailLog = ref<PaycheckLogRow | null>(null);
const emailOpen = ref(false);

function openEmail(log: PaycheckLogRow): void {
	emailLog.value = log;
	emailOpen.value = true;
}

function onPrintPaystub(): void {
	if (editPaystub.value) printer.value?.print(editPaystub.value);
}

function cancelEdit(): void {
	editDialog.value = false;
	editingLog.value = null;
}

async function onDelete(log: PaycheckLogRow): Promise<void> {
	const confirmed = await confirmDialog.confirm({
		title: t('payment.confirmDeleteTitle'),
		text: t('payment.confirmDelete'),
		confirmText: t('common.delete'),
		color: 'error',
	});
	if (!confirmed) return;

	try {
		await apiRequest(`/api/paycheck-logs/${log.id}`, { method: 'DELETE' });
	} catch (error) {
		toast.fromError(error);
		return;
	}
	results.value = results.value.filter((r) => r.id !== log.id);
	toast.success(t('payment.deleted'));
}
</script>

<template>
	<v-container>
		<h1 class="text-h5 mb-4">{{ t('nav.editPayment') }}</h1>

		<v-row dense class="mb-2">
			<v-col cols="12" md="4">
				<v-select
					v-model="employeeId"
					:items="employees"
					:item-title="employeeDisplayName"
					item-value="id"
					:label="t('dashboard.selectEmployee')"
					:placeholder="t('common.allEmployees')"
					clearable
					hide-details
				/>
			</v-col>
			<v-col cols="6" md="3">
				<v-text-field v-model="from" :label="t('common.from')" type="date" hide-details />
			</v-col>
			<v-col cols="6" md="3">
				<v-text-field v-model="to" :label="t('common.to')" type="date" hide-details />
			</v-col>
			<v-col cols="12" md="2" class="d-flex align-center">
				<v-btn block color="primary" size="large" @click="search">{{
					t('common.search')
				}}</v-btn>
			</v-col>
		</v-row>

		<v-list v-if="xs" class="rounded" lines="two">
			<v-list-item v-for="log in pagedResults" :key="log.id">
				<v-list-item-title>{{ log.firstName }} {{ log.lastName }}</v-list-item-title>
				<v-list-item-subtitle>
					{{ formatMonthDay(log.weekStart, locale) }} –
					{{ formatMonthDay(log.weekEnd, locale) }} · {{ t('payment.payday') }}
					{{ formatMonthDay(log.payday, locale) }}
				</v-list-item-subtitle>
				<div class="d-flex align-center ga-2 mt-1">
					<strong>{{ formatCurrency(log.employeeCut) }}</strong>
					<span class="text-body-2 text-medium-emphasis">{{
						t(`payment.statuses.${log.status}`)
					}}</span>
					<v-chip
						v-if="log.kind === 'lump_sum'"
						size="x-small"
						variant="outlined"
						color="primary"
						>{{ t('payment.lumpSum') }}</v-chip
					>
				</div>
				<template #append>
					<div class="d-flex flex-column">
						<v-btn
							icon="mdi-pencil"
							size="small"
							variant="text"
							:aria-label="t('common.edit')"
							@click="openEdit(log)"
						/>
						<v-btn
							v-if="log.status === 'completed'"
							icon="mdi-email-outline"
							size="small"
							variant="text"
							:aria-label="t('emailPaystub.title')"
							@click="openEmail(log)"
						/>
						<v-btn
							icon="mdi-delete"
							size="small"
							variant="text"
							:aria-label="t('common.delete')"
							@click="onDelete(log)"
						/>
					</div>
				</template>
			</v-list-item>
		</v-list>
		<v-table v-else density="compact" striped="odd">
			<thead>
				<tr>
					<th>{{ t('employees.firstName') }} {{ t('employees.lastName') }}</th>
					<th>{{ t('payment.weekStart') }}</th>
					<th>{{ t('payment.weekEnd') }}</th>
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
						{{ t(`payment.statuses.${log.status}`) }}
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
							v-if="log.status === 'completed'"
							icon="mdi-email-outline"
							size="small"
							variant="text"
							:aria-label="t('emailPaystub.title')"
							@click="openEmail(log)"
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

		<div class="d-flex flex-wrap align-center ga-2 ga-sm-4 mt-4">
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

		<v-dialog
			:model-value="editDialog"
			max-width="900"
			:fullscreen="xs"
			scrollable
			@update:model-value="
				(value: boolean) => (value ? (editDialog = true) : requestCloseEdit())
			"
		>
			<v-card :title="t('payment.editTitle')">
				<!-- Scrollable, so it clips: room for the first outlined field's floating label. -->
				<v-card-text class="pt-4">
					<v-alert v-if="editError" type="error" density="compact" class="mb-4">{{
						editError
					}}</v-alert>
					<v-row>
						<v-col cols="6" md="4">
							<v-text-field
								v-model="editForm.weekStart"
								:label="t('payment.weekStart')"
								type="date"
								hide-details
							/>
						</v-col>
						<v-col cols="6" md="4">
							<v-text-field
								v-model="editForm.weekEnd"
								:label="t('payment.weekEnd')"
								type="date"
								hide-details
							/>
						</v-col>
						<v-col cols="12" md="4">
							<v-text-field
								v-model="editForm.payday"
								:label="t('payment.payday')"
								type="date"
								hide-details
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
							<!-- Flat on a phone: a card inside the dialog's card doubles the side padding. -->
							<v-card
								:title="t('payment.addManualLoad')"
								:flat="xs"
								:class="{ 'flat-load-card': xs }"
							>
								<v-card-text :class="{ 'px-0': xs }">
									<LoadForm
										ref="editLoadForm"
										button-variant="outlined"
										@add-load="onEditAddLoad"
									/>
								</v-card-text>
							</v-card>
						</v-col>
						<v-col cols="12" md="6" class="d-flex flex-column ga-4">
							<LoadList
								v-if="!editIsLumpSum"
								:loads="editLoads"
								@remove="onEditRemoveLoad"
							/>

							<v-card>
								<v-card-text>
									<div class="d-flex justify-space-between py-1">
										<span>{{ t('payment.summaryTotal') }}</span
										><strong>{{ formatCurrency(editSummaryTotal) }}</strong>
									</div>
									<div class="d-flex justify-space-between py-1">
										<span>{{
											t('payment.employeeCut', { rate: editRatePercent })
										}}</span
										><strong>{{ formatCurrency(editCut.employeeCut) }}</strong>
									</div>
									<div class="d-flex justify-space-between py-1">
										<span>{{ t('payment.totalPay') }}</span
										><strong>{{ formatCurrency(editCut.roundedPay) }}</strong>
									</div>
								</v-card-text>
							</v-card>
						</v-col>
					</v-row>
				</v-card-text>
				<v-divider />
				<!-- Two groups: what you can do with the pay stub (left), and the dialog's own
				     decision (right). On a phone each group is a row of equal-width buttons. -->
				<v-card-actions class="edit-actions" :class="{ stacked: xs }">
					<div class="action-group">
						<v-btn
							variant="text"
							prepend-icon="mdi-file-document-outline"
							:stacked="xs"
							:size="xs ? 'small' : undefined"
							@click="paystubOpen = true"
							>{{ t('payment.viewPaystub') }}</v-btn
						>
						<v-btn
							variant="text"
							prepend-icon="mdi-printer"
							:stacked="xs"
							:size="xs ? 'small' : undefined"
							@click="onPrintPaystub"
							>{{ t('payment.print') }}</v-btn
						>
						<!-- The email sends the saved log, so it waits for unsaved edits. -->
						<v-tooltip
							v-if="editingLog?.status === 'completed'"
							:text="t('emailPaystub.saveFirst')"
							:disabled="!editIsDirty"
							location="top"
						>
							<template #activator="{ props: tooltip }">
								<span v-bind="tooltip" class="d-flex">
									<v-btn
										variant="text"
										prepend-icon="mdi-email-outline"
										:stacked="xs"
										:size="xs ? 'small' : undefined"
										:disabled="editIsDirty"
										class="flex-grow-1"
										@click="openEmail(editingLog)"
										>{{ t('emailPaystub.short') }}</v-btn
									>
								</span>
							</template>
						</v-tooltip>
					</div>
					<!-- Vuetify right-aligns dialog actions; this pushes the stub group left. -->
					<v-spacer v-if="!xs" />
					<div class="action-group">
						<v-btn variant="text" @click="requestCloseEdit">{{
							t('common.cancel')
						}}</v-btn>
						<!-- A draft's main action is Complete; a completed log's is Save. -->
						<v-btn
							:variant="editingLog?.status === 'draft' ? 'tonal' : 'flat'"
							color="primary"
							@click="saveEdit()"
							>{{ t('common.save') }}</v-btn
						>
						<v-btn
							v-if="editingLog?.status === 'draft'"
							variant="flat"
							color="success"
							@click="saveEdit(true)"
							>{{ t('payment.complete') }}</v-btn
						>
					</div>
				</v-card-actions>
			</v-card>
		</v-dialog>

		<PaystubPreviewDialog
			v-if="editPaystub"
			v-model="paystubOpen"
			:stub="editPaystub"
			@print="onPrintPaystub"
		/>
		<PaystubPrinter ref="printer" />
		<EmailPaystubDialog
			v-model="emailOpen"
			:log-id="emailLog?.id ?? null"
			:employee-id="emailLog?.employeeId ?? null"
		/>
	</v-container>
</template>

<style scoped>
.page-size {
	max-width: 8rem;
}

.edit-actions {
	flex-wrap: wrap;
	padding-inline: 16px;
}

.action-group {
	display: flex;
	gap: 8px;
}

/* Phone: two full-width rows, each split into equal buttons. */
.edit-actions.stacked {
	flex-direction: column;
	align-items: stretch;
}

.edit-actions.stacked .action-group {
	display: grid;
	grid-auto-flow: column;
	grid-auto-columns: 1fr;
}

/* Line the nested card's title up with the dialog's fields. */
.flat-load-card :deep(.v-card-item) {
	padding-inline: 0;
}
</style>

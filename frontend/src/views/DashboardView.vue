<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useEmployees } from '../composables/useEmployees';
import { formatCurrency } from '../utils/currency';
import PaymentsByEmployeeChart from '../components/dashboard/PaymentsByEmployeeChart.vue';
import PaymentsOverTimeChart from '../components/dashboard/PaymentsOverTimeChart.vue';
import { apiFetch } from '../utils/api';

type DashboardSummary = {
	year: number;
	ytdTotal: number;
	ytdGross: number;
	paycheckCount: number;
	totalLoads: number;
	byEmployee: { employeeId: string; firstName: string; lastName: string; total: number }[];
	byMonth: {
		period: string;
		employeeId: string;
		firstName: string;
		lastName: string;
		total: number;
	}[];
};

type EmployeeSummary = {
	year: number;
	employee: { id: string; firstName: string; lastName: string };
	ytdTotal: number;
	monthTotal: number;
};

const { t } = useI18n();
const { employees, loadEmployees } = useEmployees();

const summary = ref<DashboardSummary | null>(null);
const selectedEmployeeId = ref<string | null>(null);
const employeeSummary = ref<EmployeeSummary | null>(null);

async function loadSummary(): Promise<void> {
	const response = await apiFetch('/api/dashboard/summary');
	if (response.ok) summary.value = await response.json();
}

async function loadEmployeeSummary(employeeId: string): Promise<void> {
	const response = await apiFetch(`/api/dashboard/employee/${employeeId}/summary`);
	employeeSummary.value = response.ok ? await response.json() : null;
}

watch(selectedEmployeeId, (employeeId) => {
	if (employeeId) loadEmployeeSummary(employeeId);
	else employeeSummary.value = null;
});

function employeeDisplayName(employee: { firstName: string; lastName: string }): string {
	return `${employee.firstName} ${employee.lastName}`;
}

const hasByEmployeeData = computed(() => (summary.value?.byEmployee.length ?? 0) > 0);
// The current month is still in progress, so its partial total would read as a drop.
const pastMonths = computed(() => {
	const now = new Date();
	const currentPeriod = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`;
	return (summary.value?.byMonth ?? []).filter((row) => row.period !== currentPeriod);
});
const hasByMonthData = computed(() => pastMonths.value.length > 0);

onMounted(() => {
	loadEmployees();
	loadSummary();
});
</script>

<template>
	<v-container>
		<h1 class="text-h5 mb-4">{{ t('dashboard.ytdSummary') }}</h1>

		<v-row>
			<v-col cols="12" sm="6" md="3">
				<v-card>
					<v-card-text>
						<div class="text-caption text-medium-emphasis">
							{{ t('dashboard.ytdTotal', { year: summary?.year ?? '' }) }}
						</div>
						<div class="text-h5">{{ formatCurrency(summary?.ytdTotal ?? 0) }}</div>
					</v-card-text>
				</v-card>
			</v-col>
			<v-col cols="12" sm="6" md="3">
				<v-card>
					<v-card-text>
						<div class="text-caption text-medium-emphasis">
							{{ t('dashboard.ytdGross', { year: summary?.year ?? '' }) }}
						</div>
						<div class="text-h5">{{ formatCurrency(summary?.ytdGross ?? 0) }}</div>
					</v-card-text>
				</v-card>
			</v-col>
			<v-col cols="12" sm="6" md="3">
				<v-card>
					<v-card-text>
						<div class="text-caption text-medium-emphasis">
							{{ t('dashboard.paycheckCount') }}
						</div>
						<div class="text-h5">{{ summary?.paycheckCount ?? 0 }}</div>
					</v-card-text>
				</v-card>
			</v-col>
			<v-col cols="12" sm="6" md="3">
				<v-card>
					<v-card-text>
						<div class="text-caption text-medium-emphasis">
							{{ t('dashboard.totalLoads') }}
						</div>
						<div class="text-h5">{{ summary?.totalLoads ?? 0 }}</div>
					</v-card-text>
				</v-card>
			</v-col>
		</v-row>

		<v-row class="mt-2">
			<v-col cols="12" md="6">
				<v-card class="h-100" :title="t('dashboard.byEmployee')">
					<v-card-text>
						<PaymentsByEmployeeChart
							v-if="hasByEmployeeData"
							:data="summary!.byEmployee"
						/>
						<p v-else class="text-medium-emphasis">{{ t('dashboard.noData') }}</p>
					</v-card-text>
				</v-card>
			</v-col>
			<v-col cols="12" md="6">
				<v-card class="h-100" :title="t('dashboard.byTime')">
					<v-card-text>
						<PaymentsOverTimeChart v-if="hasByMonthData" :data="pastMonths" />
						<p v-else class="text-medium-emphasis">{{ t('dashboard.noData') }}</p>
					</v-card-text>
				</v-card>
			</v-col>
		</v-row>

		<v-card class="mt-4" :title="t('dashboard.employeeSummary')">
			<v-card-text>
				<v-select
					v-model="selectedEmployeeId"
					:items="employees"
					:item-title="employeeDisplayName"
					item-value="id"
					:label="t('dashboard.selectEmployee')"
				/>

				<v-row v-if="employeeSummary">
					<v-col cols="6">
						<div class="text-caption text-medium-emphasis">
							{{ t('dashboard.ytdTotal', { year: employeeSummary.year }) }}
						</div>
						<div class="text-h5">{{ formatCurrency(employeeSummary.ytdTotal) }}</div>
					</v-col>
					<v-col cols="6">
						<div class="text-caption text-medium-emphasis">
							{{ t('dashboard.monthTotal') }}
						</div>
						<div class="text-h5">{{ formatCurrency(employeeSummary.monthTotal) }}</div>
					</v-col>
				</v-row>
				<p v-else class="text-medium-emphasis">{{ t('dashboard.selectEmployeePrompt') }}</p>
			</v-card-text>
		</v-card>
	</v-container>
</template>

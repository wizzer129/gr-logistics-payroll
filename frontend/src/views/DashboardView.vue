<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useEmployees } from '../composables/useEmployees';
import { formatCurrency } from '../utils/currency';
import { formatMonthDay } from '../utils/week';
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
	logs: {
		id: string;
		weekStart: string;
		weekEnd: string;
		payday: string;
		status: string;
		summaryTotal: number;
		employeeCut: number;
	}[];
};

const { t, locale } = useI18n();
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
const hasByMonthData = computed(() => (summary.value?.byMonth.length ?? 0) > 0);

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
				<v-card :title="t('dashboard.byEmployee')">
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
				<v-card :title="t('dashboard.byTime')">
					<v-card-text>
						<PaymentsOverTimeChart v-if="hasByMonthData" :data="summary!.byMonth" />
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

				<template v-if="employeeSummary">
					<div class="text-caption text-medium-emphasis">
						{{ t('dashboard.ytdTotal', { year: employeeSummary.year }) }}
					</div>
					<div class="text-h5 mb-4">{{ formatCurrency(employeeSummary.ytdTotal) }}</div>

					<v-table density="compact" striped="odd">
						<thead>
							<tr>
								<th>{{ t('payment.weekOf') }}</th>
								<th>{{ t('payment.payday') }}</th>
								<th>Status</th>
								<th class="text-right">{{ t('payment.totalPay') }}</th>
							</tr>
						</thead>
						<tbody>
							<tr v-for="log in employeeSummary.logs" :key="log.id">
								<td>
									{{ formatMonthDay(log.weekStart, locale) }} –
									{{ formatMonthDay(log.weekEnd, locale) }}
								</td>
								<td>{{ formatMonthDay(log.payday, locale) }}</td>
								<td>{{ log.status }}</td>
								<td class="text-right">{{ formatCurrency(log.employeeCut) }}</td>
							</tr>
						</tbody>
					</v-table>
				</template>
				<p v-else class="text-medium-emphasis">{{ t('dashboard.selectEmployeePrompt') }}</p>
			</v-card-text>
		</v-card>
	</v-container>
</template>

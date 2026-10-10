<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

// TODO: wire to GET /api/dashboard/summary and /api/dashboard/employee/:id/summary
const selectedEmployeeId = ref<string | null>(null);
const employees = ref<{ id: string; firstName: string; lastName: string }[]>([]);
</script>

<template>
	<v-container>
		<h1 class="text-h5 mb-4">{{ t('dashboard.ytdSummary') }}</h1>

		<v-row>
			<v-col cols="12" md="6">
				<v-card title="Payments by Employee">
					<!-- TODO: pie chart (payout distribution by employee) -->
				</v-card>
			</v-col>
			<v-col cols="12" md="6">
				<v-card title="Payments Over Time">
					<!-- TODO: bar/line chart (payments over time) -->
				</v-card>
			</v-col>
		</v-row>

		<v-card class="mt-4" :title="t('dashboard.employeeSummary')">
			<v-card-text>
				<v-select
					v-model="selectedEmployeeId"
					:items="employees"
					item-title="lastName"
					item-value="id"
					:label="t('dashboard.selectEmployee')"
				/>
				<!-- TODO: selected employee's individual summary -->
			</v-card-text>
		</v-card>
	</v-container>
</template>

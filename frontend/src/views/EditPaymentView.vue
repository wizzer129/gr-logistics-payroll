<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const employeeId = ref<string | null>(null);
const from = ref('');
const to = ref('');
const results = ref<{ id: string; weekStart: string; weekEnd: string; status: string }[]>([]);

async function search(): Promise<void> {
	const params = new URLSearchParams();
	if (employeeId.value) params.set('employeeId', employeeId.value);
	if (from.value) params.set('from', from.value);
	if (to.value) params.set('to', to.value);

	const response = await fetch(`/api/weekly-logs?${params}`, { credentials: 'include' });
	if (response.ok) results.value = await response.json();
}
</script>

<template>
	<v-container>
		<h1 class="text-h5 mb-4">{{ t('nav.editPayment') }}</h1>

		<v-row>
			<v-col cols="12" md="4">
				<v-text-field v-model="from" label="From" type="date" />
			</v-col>
			<v-col cols="12" md="4">
				<v-text-field v-model="to" label="To" type="date" />
			</v-col>
			<v-col cols="12" md="4" class="d-flex align-center">
				<v-btn color="primary" @click="search">Search</v-btn>
			</v-col>
		</v-row>

		<v-table>
			<thead>
				<tr>
					<th>Week Start</th>
					<th>Week End</th>
					<th>Status</th>
				</tr>
			</thead>
			<tbody>
				<tr v-for="log in results" :key="log.id">
					<td>{{ log.weekStart }}</td>
					<td>{{ log.weekEnd }}</td>
					<td>{{ log.status }}</td>
				</tr>
			</tbody>
		</v-table>
	</v-container>
</template>

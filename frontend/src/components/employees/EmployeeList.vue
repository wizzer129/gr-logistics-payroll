<script setup lang="ts">
import { useI18n } from 'vue-i18n';

export type Employee = {
	id: string;
	firstName: string;
	lastName: string;
	email?: string;
	payRatePercent: number;
};

const { t } = useI18n();
defineProps<{ employees: Employee[] }>();
const emit = defineEmits<{ edit: [employee: Employee]; delete: [employee: Employee] }>();
</script>

<template>
	<v-table>
		<thead>
			<tr>
				<th>{{ t('employees.firstName') }}</th>
				<th>{{ t('employees.lastName') }}</th>
				<th>{{ t('auth.email') }}</th>
				<th>{{ t('employees.payRatePercent') }}</th>
				<th />
			</tr>
		</thead>
		<tbody>
			<tr v-for="employee in employees" :key="employee.id">
				<td>{{ employee.firstName }}</td>
				<td>{{ employee.lastName }}</td>
				<td>{{ employee.email }}</td>
				<td>{{ employee.payRatePercent }}%</td>
				<td>
					<v-btn
						icon="mdi-pencil"
						size="small"
						variant="text"
						@click="emit('edit', employee)"
					/>
					<v-btn
						icon="mdi-delete"
						size="small"
						variant="text"
						@click="emit('delete', employee)"
					/>
				</td>
			</tr>
		</tbody>
	</v-table>
</template>

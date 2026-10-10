<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import EmployeeForm, { type EmployeeInput } from '../components/employees/EmployeeForm.vue';
import EmployeeList, { type Employee } from '../components/employees/EmployeeList.vue';

const { t } = useI18n();
const employees = ref<Employee[]>([]);

async function loadEmployees(): Promise<void> {
	const response = await fetch('/api/employees', { credentials: 'include' });
	if (response.ok) employees.value = await response.json();
}

async function onSave(input: EmployeeInput): Promise<void> {
	await fetch('/api/employees', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		credentials: 'include',
		body: JSON.stringify(input),
	});
	await loadEmployees();
}

async function onDelete(employee: Employee): Promise<void> {
	await fetch(`/api/employees/${employee.id}`, { method: 'DELETE', credentials: 'include' });
	await loadEmployees();
}

function onEdit(_employee: Employee): void {
	// TODO: edit flow (inline or dialog form)
}

onMounted(loadEmployees);
</script>

<template>
	<v-container>
		<h1 class="text-h5 mb-4">{{ t('nav.employees') }}</h1>

		<v-row>
			<v-col cols="12" md="4">
				<EmployeeForm @save="onSave" />
			</v-col>
			<v-col cols="12" md="8">
				<EmployeeList :employees="employees" @edit="onEdit" @delete="onDelete" />
			</v-col>
		</v-row>
	</v-container>
</template>

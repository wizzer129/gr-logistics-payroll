<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import EmployeeForm, { type EmployeeInput } from '../components/employees/EmployeeForm.vue';
import EmployeeList, { type Employee } from '../components/employees/EmployeeList.vue';
import EmployeeEditDialog from '../components/employees/EmployeeEditDialog.vue';
import { apiFetch } from '../utils/api';

const { t } = useI18n();
const employees = ref<Employee[]>([]);

async function loadEmployees(): Promise<void> {
	const response = await apiFetch('/api/employees');
	if (response.ok) employees.value = await response.json();
}

async function onSave(input: EmployeeInput): Promise<void> {
	await apiFetch('/api/employees', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(input),
	});
	await loadEmployees();
}

async function onDelete(employee: Employee): Promise<void> {
	await apiFetch(`/api/employees/${employee.id}`, { method: 'DELETE' });
	await loadEmployees();
}

const editOpen = ref(false);
const editing = ref<Employee | null>(null);
const editSaving = ref(false);
const editError = ref<string | null>(null);

function onEdit(employee: Employee): void {
	editing.value = employee;
	editError.value = null;
	editOpen.value = true;
}

async function onEditSave(id: string, input: EmployeeInput): Promise<void> {
	editSaving.value = true;
	editError.value = null;
	try {
		const response = await apiFetch(`/api/employees/${id}`, {
			method: 'PUT',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(input),
		});
		if (!response.ok) {
			const body = await response.json().catch(() => null);
			editError.value = body?.error ?? t('employees.saveFailed');
			return;
		}
		editOpen.value = false;
		await loadEmployees();
	} finally {
		editSaving.value = false;
	}
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

		<EmployeeEditDialog
			v-model="editOpen"
			:employee="editing"
			:saving="editSaving"
			:error="editError"
			@save="onEditSave"
		/>
	</v-container>
</template>

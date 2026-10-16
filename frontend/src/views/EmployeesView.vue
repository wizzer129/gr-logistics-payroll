<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import EmployeeForm, { type EmployeeInput } from '../components/employees/EmployeeForm.vue';
import EmployeeList, { type Employee } from '../components/employees/EmployeeList.vue';
import EmployeeEditDialog from '../components/employees/EmployeeEditDialog.vue';
import { useConfirmStore } from '../store/confirm-store';
import { useToastStore } from '../store/toast-store';
import { ApiError, apiJson, apiRequest } from '../utils/api';

const { t } = useI18n();
const toast = useToastStore();
const confirmDialog = useConfirmStore();
const employees = ref<Employee[]>([]);
const form = ref<InstanceType<typeof EmployeeForm> | null>(null);

async function loadEmployees(): Promise<void> {
	try {
		employees.value = await apiJson<Employee[]>('/api/employees');
	} catch (error) {
		toast.fromError(error);
	}
}

async function onSave(input: EmployeeInput): Promise<void> {
	try {
		await apiRequest('/api/employees', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(input),
		});
	} catch (error) {
		toast.fromError(error);
		return;
	}
	form.value?.reset();
	toast.success(t('employees.created'));
	await loadEmployees();
}

async function onDelete(employee: Employee): Promise<void> {
	const confirmed = await confirmDialog.confirm({
		title: t('employees.confirmDeleteTitle'),
		text: t('employees.confirmDelete', {
			name: `${employee.firstName} ${employee.lastName}`,
		}),
		confirmText: t('common.delete'),
		color: 'error',
	});
	if (!confirmed) return;

	try {
		await apiRequest(`/api/employees/${employee.id}`, { method: 'DELETE' });
	} catch (error) {
		toast.fromError(error);
		return;
	}
	toast.success(t('employees.deleted'));
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
		await apiRequest(`/api/employees/${id}`, {
			method: 'PUT',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(input),
		});
		editOpen.value = false;
		toast.success(t('employees.updated'));
		await loadEmployees();
	} catch (error) {
		// Shown in the dialog, which stays open so the edit isn't lost.
		if (!(error instanceof ApiError)) throw error;
		editError.value = error.message;
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
				<EmployeeForm ref="form" @save="onSave" />
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

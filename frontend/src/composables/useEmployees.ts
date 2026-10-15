import { ref } from 'vue';
import { apiFetch } from '../utils/api';

export type EmployeeOption = {
	id: string;
	firstName: string;
	lastName: string;
	payRatePercent: number;
};

export function useEmployees() {
	const employees = ref<EmployeeOption[]>([]);

	async function loadEmployees(): Promise<void> {
		const response = await apiFetch('/api/employees');
		if (response.ok) employees.value = await response.json();
	}

	return { employees, loadEmployees };
}

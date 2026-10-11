import { ref } from 'vue';

export type EmployeeOption = {
	id: string;
	firstName: string;
	lastName: string;
	payRatePercent: number;
};

export function useEmployees() {
	const employees = ref<EmployeeOption[]>([]);

	async function loadEmployees(): Promise<void> {
		const response = await fetch('/api/employees', { credentials: 'include' });
		if (response.ok) employees.value = await response.json();
	}

	return { employees, loadEmployees };
}

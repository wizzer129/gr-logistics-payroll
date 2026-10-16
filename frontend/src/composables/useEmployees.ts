import { ref } from 'vue';
import { useToastStore } from '../store/toast-store';
import { apiJson } from '../utils/api';

export type EmployeeOption = {
	id: string;
	firstName: string;
	lastName: string;
	payRatePercent: number;
};

export function useEmployees() {
	const employees = ref<EmployeeOption[]>([]);
	const toast = useToastStore();

	async function loadEmployees(): Promise<void> {
		try {
			employees.value = await apiJson<EmployeeOption[]>('/api/employees');
		} catch (error) {
			toast.fromError(error);
		}
	}

	return { employees, loadEmployees };
}

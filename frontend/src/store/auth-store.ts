import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

type User = { id: string; email: string; role: 'admin' | 'employee' };

export const useAuthStore = defineStore('auth', () => {
	const user = ref<User | null>(null);

	const isAuthenticated = computed(() => user.value !== null);
	const isAdmin = computed(() => user.value?.role === 'admin');

	async function login(email: string, password: string): Promise<void> {
		const response = await fetch('/api/auth/login', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			credentials: 'include',
			body: JSON.stringify({ email, password }),
		});

		if (!response.ok) throw new Error('Invalid email or password');

		user.value = await response.json();
	}

	async function logout(): Promise<void> {
		await fetch('/api/auth/logout', { method: 'POST', credentials: 'include' });
		user.value = null;
	}

	return { user, isAuthenticated, isAdmin, login, logout };
});

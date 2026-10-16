import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { apiFetch, apiJson } from '../utils/api';

export type User = { id: string; email: string; role: 'admin' | 'employee' };

const STORAGE_KEY = 'gr-logistics-user';

function readCachedUser(): User | null {
	const cached = localStorage.getItem(STORAGE_KEY);
	if (!cached) return null;

	try {
		return JSON.parse(cached) as User;
	} catch {
		localStorage.removeItem(STORAGE_KEY);
		return null;
	}
}

export const useAuthStore = defineStore('auth', () => {
	// Seeded from localStorage so a page refresh doesn't flash the login screen
	// while restoreSession() reconciles with the (httpOnly, more authoritative) session cookie.
	const user = ref<User | null>(readCachedUser());

	const isAuthenticated = computed(() => user.value !== null);
	const isAdmin = computed(() => user.value?.role === 'admin');

	function setUser(value: User | null): void {
		user.value = value;

		if (value) localStorage.setItem(STORAGE_KEY, JSON.stringify(value));
		else localStorage.removeItem(STORAGE_KEY);
	}

	async function login(email: string, password: string): Promise<void> {
		setUser(
			await apiJson<User>('/api/auth/login', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ email, password }),
			}),
		);
	}

	/** Always logs out locally; a failed server call just leaves a session to expire. */
	async function logout(): Promise<void> {
		try {
			await apiFetch('/api/auth/logout', { method: 'POST' });
		} catch {
			// Offline: nothing to tell the server, and the user still wants out.
		} finally {
			setUser(null);
		}
	}

	/**
	 * Re-validates the cached user against the session cookie. Called once on app
	 * boot so a refresh doesn't log the user out while the cookie is still valid,
	 * and so an actually-expired session gets cleared instead of trusting the cache forever.
	 */
	async function restoreSession(): Promise<void> {
		if (!user.value) return;

		try {
			const response = await apiFetch('/api/auth/me');

			if (response.ok) setUser(await response.json());
			else setUser(null);
		} catch {
			// Network error on boot — keep the cached user rather than bouncing to login.
		}
	}

	return { user, isAuthenticated, isAdmin, login, logout, restoreSession };
});

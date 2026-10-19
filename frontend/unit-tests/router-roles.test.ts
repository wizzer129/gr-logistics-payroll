import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, test } from 'vitest';
import router from '../src/router';
import { useAuthStore, type User } from '../src/store/auth-store';

function signIn(role: User['role']): void {
	useAuthStore().user = { id: '1', email: 'x@example.com', role, locale: 'en', theme: 'default' };
}

describe('router role guard', () => {
	beforeEach(() => setActivePinia(createPinia()));

	test('a non-admin lands on /no-access instead of looping through the dashboard', async () => {
		signIn('employee');
		await router.push('/');
		expect(router.currentRoute.value.name).toBe('no-access');

		await router.push('/employees');
		expect(router.currentRoute.value.name).toBe('no-access');
	});

	test('an admin reaches the dashboard', async () => {
		signIn('admin');
		await router.push('/');
		expect(router.currentRoute.value.name).toBe('dashboard');
	});
});

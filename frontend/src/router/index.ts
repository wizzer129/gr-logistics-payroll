import { createRouter, createWebHistory } from 'vue-router';
import { useAuthStore } from '../store/auth-store';

const router = createRouter({
	history: createWebHistory(),
	routes: [
		{
			path: '/login',
			name: 'login',
			component: () => import('../views/LoginView.vue'),
			meta: { requiresAuth: false },
		},
		{
			path: '/forgot-password',
			name: 'forgot-password',
			component: () => import('../views/ForgotPasswordView.vue'),
			meta: { requiresAuth: false },
		},
		{
			// Opened from the emailed link: /reset-password?token=…
			path: '/reset-password',
			name: 'reset-password',
			component: () => import('../views/ResetPasswordView.vue'),
			meta: { requiresAuth: false },
		},
		{
			path: '/',
			name: 'dashboard',
			component: () => import('../views/DashboardView.vue'),
			meta: { requiresAuth: true, roles: ['admin'] },
		},
		{
			path: '/process-payroll',
			name: 'process-payroll',
			component: () => import('../views/ProcessPayrollView.vue'),
			meta: { requiresAuth: true, roles: ['admin'] },
		},
		{
			path: '/payroll-history',
			name: 'payroll-history',
			component: () => import('../views/PayrollHistoryView.vue'),
			meta: { requiresAuth: true, roles: ['admin'] },
		},
		// Their old names (Payment Processing, Edit Payment / View Payments): bookmarks still work.
		{ path: '/payment-processing', redirect: { name: 'process-payroll' } },
		{ path: '/edit-payment', redirect: { name: 'payroll-history' } },
		{ path: '/payments', redirect: { name: 'payroll-history' } },
		{
			path: '/settings',
			name: 'settings',
			component: () => import('../views/SettingsView.vue'),
			meta: { requiresAuth: true, roles: ['admin'] },
		},
		{
			// A signed-in user whose role has no pages (today: email-only accounts).
			path: '/no-access',
			name: 'no-access',
			component: () => import('../views/NoAccessView.vue'),
			meta: { requiresAuth: true },
		},
		{
			path: '/employees',
			name: 'employees',
			component: () => import('../views/EmployeesView.vue'),
			meta: { requiresAuth: true, roles: ['admin'] },
		},
	],
});

router.beforeEach((to) => {
	const auth = useAuthStore();

	if (to.meta.requiresAuth && !auth.isAuthenticated) {
		return { name: 'login', query: { redirect: to.fullPath } };
	}

	// Not 'dashboard': it needs admin too, so a non-admin would redirect forever.
	const roles = to.meta.roles as string[] | undefined;
	if (roles && auth.user && !roles.includes(auth.user.role)) {
		return { name: 'no-access' };
	}

	return true;
});

export default router;

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
			path: '/',
			name: 'dashboard',
			component: () => import('../views/DashboardView.vue'),
			meta: { requiresAuth: true, roles: ['admin'] },
		},
		{
			path: '/payment-processing',
			name: 'payment-processing',
			component: () => import('../views/PaymentProcessingView.vue'),
			meta: { requiresAuth: true, roles: ['admin'] },
		},
		{
			path: '/edit-payment',
			name: 'edit-payment',
			component: () => import('../views/EditPaymentView.vue'),
			meta: { requiresAuth: true, roles: ['admin'] },
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

	const roles = to.meta.roles as string[] | undefined;
	if (roles && auth.user && !roles.includes(auth.user.role)) {
		return { name: 'dashboard' };
	}

	return true;
});

export default router;

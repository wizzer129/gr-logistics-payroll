<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { useRouter } from 'vue-router';
import { useAuthStore } from './store/auth-store';
import { setLocale } from './plugins/i18n';

const { t, locale } = useI18n();
const router = useRouter();
const auth = useAuthStore();

async function onLogout(): Promise<void> {
	await auth.logout();
	await router.push({ name: 'login' });
}

function onLocaleChange(value: string): void {
	setLocale(value as 'en' | 'es');
}
</script>

<template>
	<v-app>
		<v-app-bar v-if="auth.isAuthenticated" :title="t('app.title')">
			<v-btn to="/" variant="text">{{ t('nav.dashboard') }}</v-btn>
			<v-btn to="/payment-processing" variant="text">{{ t('nav.paymentProcessing') }}</v-btn>
			<v-btn to="/edit-payment" variant="text">{{ t('nav.editPayment') }}</v-btn>
			<v-btn to="/employees" variant="text">{{ t('nav.employees') }}</v-btn>

			<v-spacer />

			<v-select
				:model-value="locale"
				:items="[
					{ title: 'EN', value: 'en' },
					{ title: 'ES', value: 'es' },
				]"
				density="compact"
				style="max-width: 100px"
				@update:model-value="onLocaleChange"
			/>
			<v-btn variant="text" @click="onLogout">{{ t('auth.logout') }}</v-btn>
		</v-app-bar>

		<v-main>
			<router-view />
		</v-main>
	</v-app>
</template>

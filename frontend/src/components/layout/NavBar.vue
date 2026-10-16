<script setup lang="ts">
import { ref } from 'vue';
import { useDisplay } from 'vuetify';
import { useI18n } from 'vue-i18n';
import { useRouter } from 'vue-router';
import { useAuthStore } from '../../store/auth-store';
import { setLocale } from '../../plugins/i18n';
import ThemeToggle from './ThemeToggle.vue';

const { t, locale } = useI18n();
const { mobile } = useDisplay();
const router = useRouter();
const auth = useAuthStore();

const drawerOpen = ref(!mobile.value);

const navLinks = [
	{ to: '/', labelKey: 'nav.dashboard' },
	{ to: '/payment-processing', labelKey: 'nav.paymentProcessing' },
	{ to: '/edit-payment', labelKey: 'nav.editPayment' },
	{ to: '/employees', labelKey: 'nav.employees' },
];

const localeItems = [
	{ title: 'EN', value: 'en' },
	{ title: 'ES', value: 'es' },
];

function onLocaleChange(value: string): void {
	setLocale(value as 'en' | 'es');
}

async function onLogout(): Promise<void> {
	await auth.logout();
	await router.push({ name: 'login' });
}
</script>

<template>
	<v-app-bar :title="t('app.title')" elevation="0">
		<template #prepend>
			<v-app-bar-nav-icon
				:aria-label="t('nav.openMenu')"
				elevation="0"
				@click="drawerOpen = !drawerOpen"
			/>
		</template>

		<v-spacer />

		<v-select
			:model-value="locale"
			:items="localeItems"
			density="compact"
			class="mt-5"
			style="max-width: 60px"
			variant="plain"
			@update:model-value="onLocaleChange"
		/>
		<ThemeToggle />
		<v-btn variant="text" @click="onLogout">{{ t('auth.logout') }}</v-btn>
	</v-app-bar>

	<v-navigation-drawer v-model="drawerOpen" :permanent="!mobile" :temporary="mobile">
		<v-list nav>
			<v-list-item
				v-for="link in navLinks"
				:key="link.to"
				:to="link.to"
				:title="t(link.labelKey)"
			/>
		</v-list>
	</v-navigation-drawer>
</template>

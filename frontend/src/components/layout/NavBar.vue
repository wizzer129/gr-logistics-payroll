<script setup lang="ts">
import { computed, ref } from 'vue';
import { useDisplay } from 'vuetify';
import { useI18n } from 'vue-i18n';
import { useRouter } from 'vue-router';
import { useAuthStore } from '../../store/auth-store';
import { setLocale } from '../../plugins/i18n';
import ThemeToggle from './ThemeToggle.vue';
import LanguageSelect from '../common/LanguageSelect.vue';

const { t, locale } = useI18n();
const { mobile } = useDisplay();
const router = useRouter();
const auth = useAuthStore();

const drawerOpen = ref(!mobile.value);

const navLinks = [
	{ to: '/', labelKey: 'nav.dashboard', icon: 'mdi-view-dashboard-outline' },
	{ to: '/payment-processing', labelKey: 'nav.paymentProcessing', icon: 'mdi-cash-plus' },
	{ to: '/edit-payment', labelKey: 'nav.editPayment', icon: 'mdi-file-document-edit-outline' },
	{ to: '/employees', labelKey: 'nav.employees', icon: 'mdi-account-group-outline' },
	{ to: '/settings', labelKey: 'nav.settings', icon: 'mdi-cog-outline' },
];

const localeItems = [
	{ title: 'EN', value: 'en' },
	{ title: 'ES', value: 'es' },
];

const currentLocale = computed(() => locale.value as 'en' | 'es');

function onLocaleChange(value: string): void {
	setLocale(value as 'en' | 'es');
	auth.saveLocale(value as 'en' | 'es');
}

async function onLogout(): Promise<void> {
	// Navigate first: a view with unsaved changes can cancel this, and then nobody's logged out.
	const failure = await router.push({ name: 'login' });
	if (failure) return;
	await auth.logout();
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

		<!-- On mobile these live at the bottom of the drawer; the bar only has room for the title. -->
		<template v-if="!mobile" #append>
			<v-select
				:model-value="locale"
				:items="localeItems"
				density="compact"
				hide-details
				class="locale-select"
				variant="plain"
				@update:model-value="onLocaleChange"
			/>
			<ThemeToggle />
			<v-btn variant="text" @click="onLogout">{{ t('auth.logout') }}</v-btn>
		</template>
	</v-app-bar>

	<v-navigation-drawer v-model="drawerOpen" :permanent="!mobile" :temporary="mobile">
		<v-list nav>
			<v-list-item
				v-for="link in navLinks"
				:key="link.to"
				:to="link.to"
				:prepend-icon="link.icon"
				:prepend-gap="16"
				:title="t(link.labelKey)"
			/>
		</v-list>

		<template v-if="mobile" #append>
			<div class="pa-4 d-flex flex-column ga-3">
				<div class="d-flex align-center ga-2">
					<LanguageSelect
						:model-value="currentLocale"
						:label="t('nav.language')"
						density="compact"
						hide-details
						class="flex-grow-1"
						@update:model-value="onLocaleChange"
					/>
					<ThemeToggle />
				</div>
				<v-btn variant="outlined" block prepend-icon="mdi-logout" @click="onLogout">{{
					t('auth.logout')
				}}</v-btn>
			</div>
		</template>
	</v-navigation-drawer>
</template>

<style scoped>
.locale-select {
	width: 60px;
	flex: none;
}
</style>

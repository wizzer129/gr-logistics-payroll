<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { useRouter } from 'vue-router';
import { useAuthStore } from '../store/auth-store';
import ThemeToggle from '../components/layout/ThemeToggle.vue';

// Where a signed-in non-admin lands. Today that's an email-only account (EMAIL-SPEC.md §6):
// it exists to receive paystubs or reports, and every data page needs the admin role.
const { t } = useI18n();
const router = useRouter();
const auth = useAuthStore();

async function onLogout(): Promise<void> {
	await auth.logout();
	await router.push({ name: 'login' });
}
</script>

<template>
	<ThemeToggle class="position-fixed top-0 right-0 ma-2" />
	<v-container class="fill-height" max-width="480">
		<v-card class="w-100 pa-2 pa-sm-4" :title="t('noAccess.title')" rounded="lg">
			<v-card-text>
				<p class="mb-2">{{ t('noAccess.body') }}</p>
				<p class="text-medium-emphasis">{{ auth.user?.email }}</p>
			</v-card-text>
			<v-card-actions>
				<v-btn color="primary" variant="flat" block @click="onLogout">{{
					t('auth.logout')
				}}</v-btn>
			</v-card-actions>
		</v-card>
	</v-container>
</template>

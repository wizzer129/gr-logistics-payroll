<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useToastStore } from '../store/toast-store';
import { apiRequest } from '../utils/api';
import ThemeToggle from '../components/layout/ThemeToggle.vue';

const { t, locale } = useI18n();
const toast = useToastStore();

const email = ref('');
const loading = ref(false);
const sent = ref(false);

async function onSubmit(): Promise<void> {
	loading.value = true;
	try {
		// The backend answers the same whether or not the account exists, so this page
		// never says which; the email goes out in the language this page is in.
		await apiRequest('/api/auth/password-reset', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ email: email.value, locale: locale.value }),
		});
		sent.value = true;
	} catch (error) {
		toast.fromError(error);
	} finally {
		loading.value = false;
	}
}
</script>

<template>
	<ThemeToggle class="position-fixed top-0 right-0 ma-2" />
	<v-container class="fill-height" max-width="420">
		<v-card class="w-100 pa-2 pa-sm-4" :title="t('auth.forgotTitle')" rounded="lg">
			<v-card-text v-if="sent">
				<v-alert type="success" variant="tonal">{{ t('auth.linkSent') }}</v-alert>
			</v-card-text>
			<v-form v-else @submit.prevent="onSubmit">
				<v-card-text>
					<p class="mb-4 text-medium-emphasis">{{ t('auth.forgotIntro') }}</p>
					<v-text-field
						v-model="email"
						:label="t('auth.email')"
						type="email"
						autocomplete="email"
						required
					/>
				</v-card-text>
				<v-card-actions>
					<v-btn
						type="submit"
						color="primary"
						block
						variant="flat"
						:loading="loading"
						:disabled="loading || !email"
						>{{ t('auth.sendLink') }}</v-btn
					>
				</v-card-actions>
			</v-form>
			<div class="text-center">
				<router-link :to="{ name: 'login' }" class="text-body-2">{{
					t('auth.backToLogin')
				}}</router-link>
			</div>
		</v-card>
	</v-container>
</template>

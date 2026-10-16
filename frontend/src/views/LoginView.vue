<script setup lang="ts">
import { ref } from 'vue';
import { useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { useAuthStore } from '../store/auth-store';
import { ApiError } from '../utils/api';
import ThemeToggle from '../components/layout/ThemeToggle.vue';
import PasswordField from '../components/common/PasswordField.vue';

const { t } = useI18n();
const router = useRouter();
const auth = useAuthStore();

const email = ref('');
const password = ref('');
const error = ref('');
const loading = ref(false);

async function onSubmit(): Promise<void> {
	error.value = '';
	loading.value = true;

	try {
		await auth.login(email.value, password.value);
		await router.push({ name: 'dashboard' });
	} catch (e) {
		// A non-API failure (offline, CORS) has an untranslated browser message.
		error.value = e instanceof ApiError ? e.message : t('auth.loginFailed');
	} finally {
		loading.value = false;
	}
}
</script>

<template>
	<ThemeToggle class="position-fixed top-0 right-0 ma-2" />
	<v-container class="fill-height" max-width="420">
		<v-card class="w-100 pa-4" :title="t('app.title')" rounded="lg">
			<v-form @submit.prevent="onSubmit">
				<v-card-text>
					<v-text-field v-model="email" :label="t('auth.email')" type="email" required />
					<PasswordField
						v-model="password"
						:label="t('auth.password')"
						autocomplete="current-password"
						required
					/>
					<v-alert v-if="error" type="error" density="compact" class="mb-2">{{
						error
					}}</v-alert>
				</v-card-text>
				<v-card-actions>
					<v-btn
						type="submit"
						color="primary"
						block
						variant="flat"
						:loading="loading"
						:disabled="loading"
						>{{ t('auth.login') }}</v-btn
					>
				</v-card-actions>
				<div class="text-center">
					<router-link :to="{ name: 'forgot-password' }" class="text-body-2">{{
						t('auth.forgotPassword')
					}}</router-link>
				</div>
			</v-form>
		</v-card>
	</v-container>
</template>

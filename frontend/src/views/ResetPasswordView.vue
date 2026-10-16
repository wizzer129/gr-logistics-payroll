<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRoute, useRouter } from 'vue-router';
import { useToastStore } from '../store/toast-store';
import { ApiError, apiRequest } from '../utils/api';
import { isValidPasswordLength } from '../utils/password';
import ThemeToggle from '../components/layout/ThemeToggle.vue';
import PasswordField from '../components/common/PasswordField.vue';

const { t } = useI18n();
const route = useRoute();
const router = useRouter();
const toast = useToastStore();

const token = ref('');
const linkInvalid = ref(false);
const password = ref('');
const confirm = ref('');
const loading = ref(false);
const form = ref<{ validate: () => Promise<{ valid: boolean }> } | null>(null);

const lengthRule = (value: string) => isValidPasswordLength(value) || t('auth.passwordLength');
const matchRule = (value: string) => value === password.value || t('auth.passwordsDontMatch');

onMounted(() => {
	const fromLink = route.query.token;
	token.value = typeof fromLink === 'string' ? fromLink : '';
	linkInvalid.value = !token.value;
	// Take the token out of the address bar, so it doesn't sit in browser history or end
	// up in a screenshot.
	if (fromLink !== undefined) router.replace({ query: {} });
});

async function onSubmit(): Promise<void> {
	if (!(await form.value?.validate())?.valid) return;

	loading.value = true;
	try {
		await apiRequest('/api/auth/password-reset/confirm', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ token: token.value, newPassword: password.value }),
		});
		toast.success(t('auth.passwordReset'));
		await router.push({ name: 'login' });
	} catch (error) {
		if (error instanceof ApiError && error.code === 'RESET_TOKEN_INVALID') {
			linkInvalid.value = true;
		} else {
			toast.fromError(error);
		}
	} finally {
		loading.value = false;
	}
}
</script>

<template>
	<ThemeToggle class="position-fixed top-0 right-0 ma-2" />
	<v-container class="fill-height" max-width="420">
		<v-card class="w-100 pa-4" :title="t('auth.resetTitle')" rounded="lg">
			<v-card-text v-if="linkInvalid">
				<v-alert type="error" variant="tonal" class="mb-4">{{
					t('auth.linkInvalid')
				}}</v-alert>
				<v-btn :to="{ name: 'forgot-password' }" color="primary" block variant="flat">{{
					t('auth.requestNewLink')
				}}</v-btn>
			</v-card-text>
			<v-form v-else ref="form" @submit.prevent="onSubmit">
				<v-card-text>
					<PasswordField
						v-model="password"
						:label="t('auth.newPassword')"
						:rules="[lengthRule]"
						:hint="t('auth.passwordLength')"
						autocomplete="new-password"
					/>
					<PasswordField
						v-model="confirm"
						:label="t('auth.confirmPassword')"
						:rules="[matchRule]"
						autocomplete="new-password"
					/>
				</v-card-text>
				<v-card-actions>
					<v-btn
						type="submit"
						color="primary"
						block
						variant="flat"
						:loading="loading"
						:disabled="loading"
						>{{ t('auth.savePassword') }}</v-btn
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

<script setup lang="ts">
import { ref } from 'vue';
import { useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { useAuthStore } from '../store/auth-store';
import ThemeToggle from '../components/layout/ThemeToggle.vue';

const { t } = useI18n();
const router = useRouter();
const auth = useAuthStore();

const email = ref('');
const password = ref('');
const showPassword = ref(false);
const error = ref('');
const loading = ref(false);

async function onSubmit(): Promise<void> {
	error.value = '';
	loading.value = true;

	try {
		await auth.login(email.value, password.value);
		await router.push({ name: 'dashboard' });
	} catch {
		error.value = t('auth.login') + ' failed';
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
					<v-text-field
						v-model="password"
						:label="t('auth.password')"
						:type="showPassword ? 'text' : 'password'"
						required
					>
						<template #append-inner>
							<v-btn
								:icon="showPassword ? 'mdi-eye-off' : 'mdi-eye'"
								:aria-label="
									t(showPassword ? 'auth.hidePassword' : 'auth.showPassword')
								"
								variant="text"
								density="compact"
								size="small"
								@click="showPassword = !showPassword"
							/>
						</template>
					</v-text-field>
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
			</v-form>
		</v-card>
	</v-container>
</template>

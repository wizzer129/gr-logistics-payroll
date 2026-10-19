<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useAuthStore } from '../../store/auth-store';
import { useToastStore } from '../../store/toast-store';
import { currentFamily, currentMode } from '../../plugins/vuetify';
import { THEME_FAMILIES, themeName, type ThemeFamily, type ThemeMode } from '../../plugins/themes';

/**
 * Pick a color theme, try it in the preview (with its own light/dark switch), then apply it.
 * Applying saves it for the user; the app bar toggle still picks light or dark.
 */
const { t } = useI18n();
const auth = useAuthStore();
const toast = useToastStore();

const selected = ref<ThemeFamily>(currentFamily());
const previewMode = ref<ThemeMode>(currentMode());
const applying = ref(false);

const items = computed(() =>
	THEME_FAMILIES.map((family) => ({ title: t(`theme.families.${family}`), value: family })),
);
const previewTheme = computed(() => themeName(selected.value, previewMode.value));
const isDirty = computed(() => selected.value !== currentFamily());

defineExpose({ isDirty });

async function apply(): Promise<void> {
	applying.value = true;
	try {
		await auth.saveTheme(selected.value);
		toast.success(t('theme.applied'));
	} catch (error) {
		toast.fromError(error);
	} finally {
		applying.value = false;
	}
}

const SWATCHES = ['primary', 'secondary', 'success', 'info', 'warning', 'error'] as const;
</script>

<template>
	<v-card :title="t('theme.title')">
		<v-card-text>
			<v-row>
				<v-col cols="12" md="4">
					<p class="mb-4 text-medium-emphasis">{{ t('theme.hint') }}</p>
					<v-select v-model="selected" :items="items" :label="t('theme.label')" />
					<v-btn
						color="primary"
						variant="flat"
						block
						:loading="applying"
						:disabled="applying || !isDirty"
						@click="apply"
						>{{ t('theme.apply') }}</v-btn
					>
				</v-col>

				<v-col cols="12" md="8">
					<div class="d-flex align-center justify-space-between mb-2">
						<span class="text-subtitle-2">{{ t('theme.preview') }}</span>
						<v-btn-toggle
							v-model="previewMode"
							mandatory
							density="compact"
							variant="outlined"
							divided
						>
							<v-btn
								value="light"
								prepend-icon="mdi-weather-sunny"
								:aria-label="t('theme.light')"
								>{{ t('theme.light') }}</v-btn
							>
							<v-btn
								value="dark"
								prepend-icon="mdi-weather-night"
								:aria-label="t('theme.dark')"
								>{{ t('theme.dark') }}</v-btn
							>
						</v-btn-toggle>
					</div>

					<!-- Renders in the picked theme only; the rest of the page keeps the applied one. -->
					<v-theme-provider :theme="previewTheme" with-background class="preview">
						<v-toolbar color="surface" density="compact" class="px-2">
							<v-icon icon="mdi-menu" class="mr-3" />
							<v-toolbar-title class="text-body-1">{{
								t('app.title')
							}}</v-toolbar-title>
							<v-icon icon="mdi-weather-night" />
						</v-toolbar>

						<div class="pa-4 d-flex flex-column ga-4">
							<div class="d-flex flex-wrap ga-2">
								<div
									v-for="swatch in SWATCHES"
									:key="swatch"
									class="swatch"
									:class="`bg-${swatch}`"
									:title="swatch"
								/>
							</div>

							<v-card
								:title="t('theme.sampleTitle')"
								:subtitle="t('theme.sampleSubtitle')"
							>
								<v-card-text class="d-flex flex-column ga-3">
									<div class="d-flex justify-space-between">
										<span>{{ t('payment.summaryTotal') }}</span
										><strong>$1,250.00</strong>
									</div>
									<div class="d-flex flex-wrap ga-2">
										<v-chip color="success" size="small" label>{{
											t('payment.statuses.completed')
										}}</v-chip>
										<v-chip color="warning" size="small" label>{{
											t('payment.statuses.draft')
										}}</v-chip>
										<v-chip color="primary" size="small" variant="outlined">{{
											t('payment.lumpSum')
										}}</v-chip>
									</div>
									<v-text-field
										:label="t('payment.storeName')"
										model-value="Walmart #1234"
										hide-details
									/>
									<v-alert type="info" density="compact" variant="tonal">{{
										t('theme.sampleAlert')
									}}</v-alert>
								</v-card-text>
								<v-card-actions class="flex-wrap">
									<v-spacer />
									<v-btn variant="text">{{ t('common.cancel') }}</v-btn>
									<v-btn color="secondary" variant="tonal">{{
										t('payment.saveDraft')
									}}</v-btn>
									<v-btn color="primary" variant="flat">{{
										t('payment.complete')
									}}</v-btn>
								</v-card-actions>
							</v-card>
						</div>
					</v-theme-provider>
				</v-col>
			</v-row>
		</v-card-text>
	</v-card>
</template>

<style scoped>
.preview {
	display: block;
	border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
	border-radius: 8px;
	overflow: hidden;
}

.swatch {
	width: 32px;
	height: 32px;
	border-radius: 6px;
}
</style>

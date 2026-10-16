<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Load, PaymentMode } from '../../store/payment-processing-store';
import type { ParsedLoadsCsv } from '../../utils/csv-loads';
import CsvLoadUpload from './CsvLoadUpload.vue';
import LoadForm from './LoadForm.vue';

const { t } = useI18n();
defineProps<{ payRatePercent: number }>();
const emit = defineEmits<{
	'add-load': [load: Load];
	'csv-processed': [csv: ParsedLoadsCsv];
	'csv-cleared': [];
}>();

const mode = defineModel<PaymentMode>('mode', { required: true });
const lumpSum = defineModel<number>('lumpSum', { required: true });

// Blank shows as an empty field rather than "0"; anything non-numeric counts as 0.
const lumpSumInput = computed({
	get: () => lumpSum.value || '',
	set: (value: string | number) => {
		lumpSum.value = Number(value) || 0;
	},
});

const tab = ref<'upload' | 'manual' | 'lumpSum'>('upload');

const loadForm = ref<InstanceType<typeof LoadForm> | null>(null);
defineExpose({ isDirty: computed(() => Boolean(loadForm.value?.isDirty)) });
// The open tab decides what gets paid: the Lump Sum tab pays the lump sum, the other two
// pay the loads.
watch(tab, (value) => {
	mode.value = value === 'lumpSum' ? 'lumpSum' : 'loads';
});
</script>

<template>
	<div class="folder">
		<v-tabs v-model="tab" class="folder-tabs" color="primary" hide-slider>
			<v-tab value="upload">{{ t('payment.uploadLoads') }}</v-tab>
			<v-tab value="manual">{{ t('payment.addManualLoad') }}</v-tab>
			<v-tab value="lumpSum">{{ t('payment.lumpSum') }}</v-tab>
		</v-tabs>
		<v-card class="folder-body" elevation="2">
			<v-card-text>
				<v-window v-model="tab">
					<v-window-item value="upload">
						<CsvLoadUpload
							:pay-rate-percent="payRatePercent"
							@processed="emit('csv-processed', $event)"
							@cleared="emit('csv-cleared')"
						/>
					</v-window-item>
					<v-window-item value="manual">
						<LoadForm ref="loadForm" @add-load="emit('add-load', $event)" />
					</v-window-item>
					<v-window-item value="lumpSum">
						<v-text-field
							v-model="lumpSumInput"
							:label="t('payment.lumpSumAmount')"
							:hint="t('payment.lumpSumHint')"
							persistent-hint
							type="number"
							prefix="$"
							min="0"
							step="0.01"
						/>
					</v-window-item>
				</v-window>
			</v-card-text>
		</v-card>
	</div>
</template>

<style scoped>
/* Folder tabs: the selected tab is raised like the card and shares its background, with no
   bottom edge or bottom shadow, so tab and card read as one piece. Unselected tabs sit
   behind, flat and tinted. */
.folder {
	--folder-radius: 8px;
	/* Vuetify's elevation-2, so the selected tab matches the card. */
	--folder-shadow:
		0 3px 1px -2px var(--v-shadow-key-umbra-opacity, rgba(0, 0, 0, 0.985)),
		0 2px 2px 0 var(--v-shadow-key-penumbra-opacity, rgba(0, 0, 0, 0.923)),
		0 1px 5px 0 var(--v-shadow-key-ambient-opacity, rgba(0, 0, 0, 0.746));
}

/* Above the card, so the card's shadow doesn't paint over the selected tab's bottom edge. */
.folder-tabs {
	position: relative;
	z-index: 1;
}

/* Let the selected tab's shadow show past the tab row instead of being clipped. */
.folder-tabs :deep(.v-slide-group__container) {
	overflow: visible;
	contain: none;
}

.folder-tabs :deep(.v-tab.v-tab.v-btn) {
	border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
	border-bottom: none;
	border-radius: var(--folder-radius) var(--folder-radius) 0 0;
	background: rgba(var(--v-theme-on-surface), 0.06);
	text-transform: none;
	letter-spacing: normal;
	/* Three tabs must fit the narrow column beside the history gutter. */
	min-width: 0;
	padding-inline: 12px;
}

/* Adjacent tabs share one border instead of doubling it. */
.folder-tabs :deep(.v-tab + .v-tab) {
	margin-left: -1px;
}

.folder-tabs :deep(.v-tab.v-tab.v-btn.v-tab--selected) {
	z-index: 1;
	border-color: transparent;
	background: rgb(var(--v-theme-surface));
	box-shadow: var(--folder-shadow);
	/* Keep the shadow off the bottom, where the tab joins the card. */
	clip-path: inset(-8px -8px 0 -8px);
}

/* No active-state tint on the selected tab — it should match the card exactly. */
.folder-tabs :deep(.v-tab--selected .v-btn__overlay) {
	opacity: 0;
}

/* v-window clips its overflow; leave room for an outlined field's floating label. */
.folder-body :deep(.v-window-item) {
	padding-top: 8px;
}

.folder-body {
	border-top-left-radius: 0;
	border-top-right-radius: var(--folder-radius);
	border-bottom-left-radius: var(--folder-radius);
	border-bottom-right-radius: var(--folder-radius);
}
</style>

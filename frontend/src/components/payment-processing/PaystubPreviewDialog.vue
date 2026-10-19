<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useDisplay } from 'vuetify';
import PaystubSheet from './PaystubSheet.vue';
import type { PaystubData } from './Paystub.vue';

// The on-screen pay stub, as it will print. Printing itself is the parent's PaystubPrinter.
const open = defineModel<boolean>({ required: true });
defineProps<{ stub: PaystubData }>();
const emit = defineEmits<{ print: [] }>();

const { t } = useI18n();

// The paper is 7.5in (720px) wide; on a narrower screen, shrink it to fit rather than cut it
// off. 64px is the dialog's margins plus the desk's padding. Printing isn't affected.
const PAPER_WIDTH_PX = 720;
const { width } = useDisplay();
const zoom = computed(() => Math.min(1, (width.value - 64) / PAPER_WIDTH_PX));
</script>

<template>
	<v-dialog v-model="open" width="auto" scrollable>
		<v-card>
			<v-card-text class="paystub-preview">
				<PaystubSheet :stub="stub" :style="{ zoom }" />
			</v-card-text>
			<v-card-actions>
				<v-spacer />
				<v-btn variant="text" @click="open = false">{{ t('common.close') }}</v-btn>
				<v-btn
					variant="flat"
					color="primary"
					prepend-icon="mdi-printer"
					@click="emit('print')"
					>{{ t('payment.print') }}</v-btn
				>
			</v-card-actions>
		</v-card>
	</v-dialog>
</template>

<style scoped>
/* A neutral "desk" around the paper so the stub reads as a document in either theme. */
.paystub-preview {
	padding: 24px !important;
	background: rgba(var(--v-theme-on-surface), 0.06);
}

/* On a phone the desk would eat the paper's width (Vuetify's xs breakpoint). */
@media (max-width: 599.98px) {
	.paystub-preview {
		padding: 8px !important;
	}
}

.paystub-preview :deep(.paystub-sheet .paystub) {
	box-shadow: 0 2px 10px rgba(0, 0, 0, 0.15);
}
</style>

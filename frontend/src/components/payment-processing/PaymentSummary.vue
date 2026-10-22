<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { useDisplay } from 'vuetify';
import { formatCurrency } from '../../utils/currency';

const { t } = useI18n();
// On a phone the three buttons stack full width instead of wrapping unevenly.
const { xs, sm, md } = useDisplay();

const props = defineProps<{
	summaryTotal: number;
	employeeCut: number;
	roundedPay: number;
	payRatePercent: number;
	hasPayment: boolean;
	canComplete: boolean;
	/** Which save is in flight, if any. */
	processing: 'draft' | 'complete' | null;
	/** Saved as a draft and still open: offer to move on to the next paycheck. */
	draftSaved?: boolean;
}>();

const emit = defineEmits<{
	complete: [];
	'save-draft': [];
	paystub: [];
	'process-next': [];
}>();
</script>

<template>
	<!-- Two cards: the totals, and (once there's something to save) what to do with them. -->
	<div :class="['d-flex', 'flex-column', xs || sm || md ? 'ga-3' : 'ga-4 mt-12']">
		<v-card>
			<v-card-text>
				<div class="d-flex justify-space-between py-1">
					<span>{{ t('payment.summaryTotal') }}</span
					><strong>{{ formatCurrency(props.summaryTotal) }}</strong>
				</div>
				<div class="d-flex justify-space-between py-1">
					<span>{{ t('payment.employeeCut', { rate: props.payRatePercent }) }}</span
					><strong>{{ formatCurrency(props.employeeCut) }}</strong>
				</div>
				<div class="d-flex justify-space-between py-1">
					<span>{{ t('payment.totalPay') }}</span
					><strong>{{ formatCurrency(props.roundedPay) }}</strong>
				</div>
			</v-card-text>
		</v-card>
		<!-- An even two-column grid: Save Draft always top left, Complete always last. Before a
		     draft is saved, Complete spans the bottom row; after, Process Next sits beside Save
		     Draft. One column on a phone. -->
		<v-card v-if="props.hasPayment || props.draftSaved">
			<v-card-actions class="summary-actions" :class="{ 'one-column': xs }">
				<v-btn
					block
					variant="outlined"
					prepend-icon="mdi-content-save-outline"
					:disabled="!props.canComplete || props.processing === 'complete'"
					:loading="props.processing === 'draft'"
					@click="emit('save-draft')"
					>{{ t('payment.saveDraft') }}</v-btn
				>
				<!-- Once a draft is saved the form stays on it; this clears it for the next one. -->
				<v-btn
					v-if="props.draftSaved"
					block
					variant="outlined"
					color="success"
					prepend-icon="mdi-skip-next-outline"
					:disabled="props.processing !== null"
					@click="emit('process-next')"
					>{{ t('payment.processNext') }}</v-btn
				>
				<v-btn
					block
					variant="outlined"
					prepend-icon="mdi-file-document-outline"
					@click="emit('paystub')"
					>{{ t('payment.viewPaystub') }}</v-btn
				>
				<v-btn
					block
					variant="outlined"
					color="primary"
					prepend-icon="mdi-check-circle-outline"
					:class="{ 'full-row': !props.draftSaved }"
					:disabled="!props.canComplete || props.processing === 'draft'"
					:loading="props.processing === 'complete'"
					@click="emit('complete')"
					>{{ t('payment.complete') }}</v-btn
				>
			</v-card-actions>
		</v-card>
	</div>
</template>

<style scoped>
.summary-actions {
	display: grid;
	grid-template-columns: repeat(2, minmax(0, 1fr));
	gap: 8px;
	padding: 16px;
}

.summary-actions.one-column {
	grid-template-columns: minmax(0, 1fr);
}

.summary-actions:not(.one-column) .full-row {
	grid-column: 1 / -1;
}

/* v-card-actions spaces buttons with margins; the grid's gap does it here. */
.summary-actions .v-btn {
	margin: 0 !important;
}
</style>

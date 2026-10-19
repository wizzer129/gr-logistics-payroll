<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { useDisplay } from 'vuetify';
import { formatCurrency } from '../../utils/currency';

const { t } = useI18n();
// On a phone the three buttons stack full width instead of wrapping unevenly.
const { xs } = useDisplay();

const props = defineProps<{
	summaryTotal: number;
	employeeCut: number;
	roundedPay: number;
	payRatePercent: number;
	hasPayment: boolean;
	canComplete: boolean;
	/** Which save is in flight, if any. */
	processing: 'draft' | 'complete' | null;
}>();

const emit = defineEmits<{ complete: []; 'save-draft': []; paystub: [] }>();
</script>

<template>
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
		<v-card-actions v-if="props.hasPayment" class="flex-wrap ga-2" :class="{ stacked: xs }">
			<v-btn
				:block="xs"
				variant="outlined"
				prepend-icon="mdi-file-document-outline"
				@click="emit('paystub')"
				>{{ t('payment.viewPaystub') }}</v-btn
			>
			<v-btn
				:block="xs"
				variant="outlined"
				prepend-icon="mdi-content-save-outline"
				:disabled="!props.canComplete || props.processing === 'complete'"
				:loading="props.processing === 'draft'"
				@click="emit('save-draft')"
				>{{ t('payment.saveDraft') }}</v-btn
			>
			<v-btn
				:block="xs"
				variant="outlined"
				color="primary"
				:disabled="!props.canComplete || props.processing === 'draft'"
				:loading="props.processing === 'complete'"
				@click="emit('complete')"
				>{{ t('payment.complete') }}</v-btn
			>
		</v-card-actions>
	</v-card>
</template>

<style scoped>
.stacked {
	flex-direction: column;
	align-items: stretch;
}
</style>

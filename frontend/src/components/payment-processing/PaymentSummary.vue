<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { formatCurrency } from '../../utils/currency';

const { t } = useI18n();

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
			<div class="d-flex justify-space-between">
				<span>{{ t('payment.summaryTotal') }}</span
				><strong>{{ formatCurrency(props.summaryTotal) }}</strong>
			</div>
			<div class="d-flex justify-space-between">
				<span>{{ t('payment.employeeCut', { rate: props.payRatePercent }) }}</span
				><strong>{{ formatCurrency(props.employeeCut) }}</strong>
			</div>
			<div class="d-flex justify-space-between">
				<span>{{ t('payment.totalPay') }}</span
				><strong>{{ formatCurrency(props.roundedPay) }}</strong>
			</div>
		</v-card-text>
		<v-card-actions v-if="props.hasPayment" class="flex-wrap ga-2">
			<v-btn
				variant="outlined"
				prepend-icon="mdi-file-document-outline"
				@click="emit('paystub')"
				>{{ t('payment.viewPaystub') }}</v-btn
			>
			<v-btn
				variant="outlined"
				prepend-icon="mdi-content-save-outline"
				:disabled="!props.canComplete || props.processing === 'complete'"
				:loading="props.processing === 'draft'"
				@click="emit('save-draft')"
				>{{ t('payment.saveDraft') }}</v-btn
			>
			<v-btn
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

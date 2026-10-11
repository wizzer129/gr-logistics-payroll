<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { formatCurrency } from '../../utils/currency';

const { t } = useI18n();

const props = defineProps<{
	summaryTotal: number;
	employeeCut: number;
	roundedPay: number;
	payRatePercent: number;
	completed: boolean;
}>();

const emit = defineEmits<{ complete: []; print: [] }>();
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
		<v-card-actions>
			<v-btn v-if="!props.completed" color="primary" @click="emit('complete')">{{
				t('payment.complete')
			}}</v-btn>
			<v-btn v-else color="secondary" @click="emit('print')">{{ t('payment.print') }}</v-btn>
		</v-card-actions>
	</v-card>
</template>

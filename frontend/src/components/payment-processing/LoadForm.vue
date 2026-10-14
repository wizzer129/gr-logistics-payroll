<script setup lang="ts">
import { reactive } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Load } from '../../store/payment-processing-store';

const { t } = useI18n();
const { buttonVariant = 'elevated' } = defineProps<{
	buttonVariant?: 'elevated' | 'outlined';
}>();
const emit = defineEmits<{ 'add-load': [load: Load] }>();

const form = reactive<Load>({ storeName: '', loadDate: '', pay: 0, extraPay: 0 });

function onSubmit(): void {
	emit('add-load', { ...form });
	form.storeName = '';
	form.loadDate = '';
	form.pay = 0;
	form.extraPay = 0;
}
</script>

<template>
	<v-form @submit.prevent="onSubmit">
		<v-text-field v-model="form.storeName" :label="t('payment.storeName')" required />
		<v-text-field v-model="form.loadDate" :label="t('payment.date')" type="date" required />
		<v-text-field
			v-model.number="form.pay"
			:label="t('payment.pay')"
			type="number"
			step="0.01"
			required
		/>
		<v-text-field
			v-model.number="form.extraPay"
			:label="t('payment.extraPay')"
			type="number"
			step="0.01"
		/>
		<v-btn type="submit" color="primary" :variant="buttonVariant">{{
			t('payment.addManualLoad')
		}}</v-btn>
	</v-form>
</template>

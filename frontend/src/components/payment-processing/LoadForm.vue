<script setup lang="ts">
import { computed, reactive } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Load } from '../../store/payment-processing-store';

const { t } = useI18n();
const { buttonVariant = 'elevated' } = defineProps<{
	buttonVariant?: 'elevated' | 'outlined';
}>();
const emit = defineEmits<{ 'add-load': [load: Load] }>();

const EMPTY: Required<Load> = { storeName: '', loadDate: '', pay: 0, extraPay: 0 };

const form = reactive<Load>({ ...EMPTY });

// A load typed in but not yet added with the button.
const isDirty = computed(() =>
	(Object.keys(EMPTY) as (keyof Load)[]).some((key) => form[key] !== EMPTY[key]),
);

defineExpose({ isDirty });

function onSubmit(): void {
	emit('add-load', { ...form });
	Object.assign(form, EMPTY);
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

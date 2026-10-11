<script setup lang="ts">
import { reactive } from 'vue';
import { useI18n } from 'vue-i18n';

export type EmployeeInput = {
	firstName: string;
	lastName: string;
	email?: string;
	payRatePercent: number;
};

const { t } = useI18n();
const emit = defineEmits<{ save: [employee: EmployeeInput] }>();

const form = reactive<EmployeeInput>({ firstName: '', lastName: '', email: '', payRatePercent: 42 });

function onSubmit(): void {
	emit('save', { ...form });
	form.firstName = '';
	form.lastName = '';
	form.email = '';
	form.payRatePercent = 42;
}
</script>

<template>
	<v-form @submit.prevent="onSubmit">
		<v-text-field v-model="form.firstName" :label="t('employees.firstName')" required />
		<v-text-field v-model="form.lastName" :label="t('employees.lastName')" required />
		<v-text-field v-model="form.email" :label="t('auth.email')" type="email" />
		<v-text-field
			v-model.number="form.payRatePercent"
			:label="t('employees.payRatePercent')"
			type="number"
			suffix="%"
			min="0"
			max="100"
			step="0.5"
		/>
		<v-btn type="submit" color="primary">{{ t('employees.add') }}</v-btn>
	</v-form>
</template>

<script setup lang="ts">
import { computed, reactive } from 'vue';
import { useI18n } from 'vue-i18n';
import LanguageSelect from '../common/LanguageSelect.vue';

export type EmployeeInput = {
	firstName: string;
	lastName: string;
	email?: string;
	payRatePercent: number;
	/** The language their emails (e.g. paystubs) go out in. */
	locale: 'en' | 'es';
};

const { t } = useI18n();
const emit = defineEmits<{ save: [employee: EmployeeInput] }>();

const EMPTY: EmployeeInput = {
	firstName: '',
	lastName: '',
	email: '',
	payRatePercent: 42,
	locale: 'en',
};

const form = reactive<EmployeeInput>({ ...EMPTY });

const isDirty = computed(() =>
	(Object.keys(EMPTY) as (keyof EmployeeInput)[]).some((key) => form[key] !== EMPTY[key]),
);

function onSubmit(): void {
	emit('save', { ...form });
}

/** Called by the parent once the save succeeded, so a failed save keeps what was typed. */
function reset(): void {
	Object.assign(form, EMPTY);
}

defineExpose({ reset, isDirty });
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
		<LanguageSelect
			v-model="form.locale"
			:label="t('employees.language')"
			:hint="t('employees.languageHint')"
			persistent-hint
			class="mb-4"
		/>
		<v-btn type="submit" color="primary" block>{{ t('employees.add') }}</v-btn>
	</v-form>
</template>

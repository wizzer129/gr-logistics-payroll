<script setup lang="ts">
import { computed, reactive, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Employee } from './EmployeeList.vue';
import type { EmployeeInput } from './EmployeeForm.vue';
import { useUnsavedChanges } from '../../composables/useUnsavedChanges';
import LanguageSelect from '../common/LanguageSelect.vue';

const props = defineProps<{ employee: Employee | null; saving: boolean; error: string | null }>();
const open = defineModel<boolean>({ required: true });
const emit = defineEmits<{ save: [id: string, input: EmployeeInput] }>();

const { t } = useI18n();

const form = reactive<EmployeeInput>({
	firstName: '',
	lastName: '',
	email: '',
	payRatePercent: 42,
	locale: 'en',
});

// Refill from the employee each time the dialog opens, so a cancelled edit leaves nothing behind.
watch(open, (isOpen) => {
	if (!isOpen || !props.employee) return;
	form.firstName = props.employee.firstName;
	form.lastName = props.employee.lastName;
	form.email = props.employee.email ?? '';
	form.payRatePercent = props.employee.payRatePercent;
	form.locale = props.employee.locale;
});

// Only while open: after a successful save the parent closes it with the old props still set.
const isDirty = computed(() => {
	const employee = props.employee;
	if (!open.value || !employee) return false;
	return (
		form.firstName !== employee.firstName ||
		form.lastName !== employee.lastName ||
		form.email !== (employee.email ?? '') ||
		form.payRatePercent !== employee.payRatePercent ||
		form.locale !== employee.locale
	);
});

const { confirmDiscard } = useUnsavedChanges();

// Cancel, Escape and clicking outside all come through here.
async function requestClose(): Promise<void> {
	if (await confirmDiscard(() => isDirty.value)) open.value = false;
}

defineExpose({ isDirty });

function onSubmit(): void {
	if (props.employee) emit('save', props.employee.id, { ...form });
}
</script>

<template>
	<v-dialog
		:model-value="open"
		max-width="480"
		:persistent="saving"
		@update:model-value="(value: boolean) => (value ? (open = true) : requestClose())"
	>
		<v-card :title="t('employees.edit')">
			<v-form @submit.prevent="onSubmit">
				<v-card-text>
					<v-alert v-if="error" type="error" density="compact" class="mb-4">{{
						error
					}}</v-alert>
					<v-text-field
						v-model="form.firstName"
						:label="t('employees.firstName')"
						required
					/>
					<v-text-field
						v-model="form.lastName"
						:label="t('employees.lastName')"
						required
					/>
					<v-text-field v-model="form.email" :label="t('auth.email')" type="email" />
					<v-text-field
						v-model.number="form.payRatePercent"
						:label="t('employees.payRatePercent')"
						:hint="t('employees.payRateHint')"
						persistent-hint
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
					/>
				</v-card-text>
				<v-card-actions>
					<v-spacer />
					<v-btn variant="outlined" :disabled="saving" @click="requestClose">{{
						t('common.cancel')
					}}</v-btn>
					<v-btn type="submit" variant="outlined" color="primary" :loading="saving">{{
						t('common.save')
					}}</v-btn>
				</v-card-actions>
			</v-form>
		</v-card>
	</v-dialog>
</template>

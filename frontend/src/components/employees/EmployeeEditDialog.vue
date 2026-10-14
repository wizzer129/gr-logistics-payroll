<script setup lang="ts">
import { reactive, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Employee } from './EmployeeList.vue';
import type { EmployeeInput } from './EmployeeForm.vue';

const props = defineProps<{ employee: Employee | null; saving: boolean; error: string | null }>();
const open = defineModel<boolean>({ required: true });
const emit = defineEmits<{ save: [id: string, input: EmployeeInput] }>();

const { t } = useI18n();

const form = reactive<EmployeeInput>({
	firstName: '',
	lastName: '',
	email: '',
	payRatePercent: 42,
});

// Refill from the employee each time the dialog opens, so a cancelled edit leaves nothing behind.
watch(open, (isOpen) => {
	if (!isOpen || !props.employee) return;
	form.firstName = props.employee.firstName;
	form.lastName = props.employee.lastName;
	form.email = props.employee.email ?? '';
	form.payRatePercent = props.employee.payRatePercent;
});

function onSubmit(): void {
	if (props.employee) emit('save', props.employee.id, { ...form });
}
</script>

<template>
	<v-dialog v-model="open" max-width="480" :persistent="saving">
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
				</v-card-text>
				<v-card-actions>
					<v-spacer />
					<v-btn variant="outlined" :disabled="saving" @click="open = false">{{
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

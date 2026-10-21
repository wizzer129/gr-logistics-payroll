<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useToastStore } from '../../store/toast-store';
import { apiJson } from '../../utils/api';
import {
	fromKeys,
	isEmail,
	toKeys,
	type RecipientOptions,
	type RecipientSelection,
} from '../../utils/recipients';

/**
 * Pick who an email goes to: users and/or employees with an email (EMAIL-SPEC.md §6), and,
 * with `allowTyped`, addresses typed in by hand (Enter, comma or space adds one).
 */
const selection = defineModel<RecipientSelection>({ required: true });
const props = defineProps<{
	label: string;
	usersOnly?: boolean;
	allowTyped?: boolean;
	/** Only these employees are offered (e.g. a bulk send's one employee); all if unset. */
	employeeIds?: string[];
	/** Starts this employee selected, if they can receive email (e.g. the paystub's own). */
	preselectEmployeeId?: string;
}>();

const { t } = useI18n();
const toast = useToastStore();
const options = ref<RecipientOptions | null>(null);
const loading = ref(false);

onMounted(async () => {
	loading.value = true;
	try {
		options.value = await apiJson<RecipientOptions>('/api/email/recipient-options');
		const preselect = props.preselectEmployeeId;
		const canReceive = employees.value.some((employee) => employee.id === preselect);
		if (preselect && canReceive && !selection.value.employeeIds.includes(preselect)) {
			selection.value = {
				...selection.value,
				employeeIds: [...selection.value.employeeIds, preselect],
			};
		}
	} catch (error) {
		toast.fromError(error);
	} finally {
		loading.value = false;
	}
});

const employees = computed(() => {
	if (!options.value || props.usersOnly) return [];
	const allowed = props.employeeIds;
	return options.value.employees.filter((employee) => !allowed || allowed.includes(employee.id));
});

const items = computed(() => {
	if (!options.value) return [];
	const users = options.value.users.map((user) => ({
		title: user.email,
		value: `user:${user.id}`,
		subtitle: t('recipients.user'),
	}));
	const employeeItems = employees.value.map((employee) => ({
		title: `${employee.firstName} ${employee.lastName}`,
		value: `employee:${employee.id}`,
		subtitle: `${t('recipients.employee')} · ${employee.email}`,
	}));
	return [...users, ...employeeItems];
});

// Show each item's kind (and an employee's address) under its name.
const itemProps = (item: { subtitle?: string }) => ({ subtitle: item.subtitle });

const keys = computed({
	get: () => toKeys(selection.value),
	set: (value: string[]) => {
		selection.value = fromKeys(value);
	},
});

const invalidTyped = computed(() => selection.value.emails.filter((email) => !isEmail(email)));
const errorMessages = computed(() =>
	invalidTyped.value.length
		? [t('recipients.invalidEmail', { emails: invalidTyped.value.join(', ') })]
		: [],
);

defineExpose({ isValid: computed(() => invalidTyped.value.length === 0) });
</script>

<template>
	<v-combobox
		v-if="allowTyped"
		v-model="keys"
		:items="items"
		:item-props="itemProps"
		:label="label"
		:loading="loading"
		:hint="t('recipients.typeHint')"
		:error-messages="errorMessages"
		:delimiters="[',', ' ', ';']"
		:return-object="false"
		persistent-hint
		multiple
		chips
		closable-chips
	/>
	<v-autocomplete
		v-else
		v-model="keys"
		:items="items"
		:item-props="itemProps"
		:label="label"
		:loading="loading"
		multiple
		chips
		closable-chips
	/>
</template>

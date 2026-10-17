<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useToastStore } from '../../store/toast-store';
import { apiJson } from '../../utils/api';
import {
	fromKeys,
	toKeys,
	type RecipientOptions,
	type RecipientSelection,
} from '../../utils/recipients';

/**
 * Pick who an email goes to: users and/or employees with an email (EMAIL-SPEC.md §6). The
 * backend turns the ids into addresses, so nothing here is a typed-in address.
 */
const selection = defineModel<RecipientSelection>({ required: true });
const props = defineProps<{
	label: string;
	usersOnly?: boolean;
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
		const canReceive = options.value.employees.some((employee) => employee.id === preselect);
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

const items = computed(() => {
	if (!options.value) return [];
	const users = options.value.users.map((user) => ({
		title: user.email,
		value: `user:${user.id}`,
		subtitle: t('recipients.user'),
	}));
	if (props.usersOnly) return users;
	const employees = options.value.employees.map((employee) => ({
		title: `${employee.firstName} ${employee.lastName}`,
		value: `employee:${employee.id}`,
		subtitle: `${t('recipients.employee')} · ${employee.email}`,
	}));
	return [...users, ...employees];
});

// Show each item's kind (and an employee's address) under its name.
const itemProps = (item: { subtitle: string }) => ({ subtitle: item.subtitle });

const keys = computed({
	get: () => toKeys(selection.value),
	set: (value: string[]) => {
		selection.value = fromKeys(value);
	},
});
</script>

<template>
	<v-autocomplete
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

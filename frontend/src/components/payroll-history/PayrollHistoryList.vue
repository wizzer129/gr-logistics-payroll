<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { formatCurrency } from '../../utils/currency';
import { formatMonthDay } from '../../utils/week';
import PaycheckRowActions from './PaycheckRowActions.vue';
import { canEmail, toggleId, type PaycheckLogRow } from './types';

/**
 * Payroll History on a phone: the table's rows as stacked list items, with only the payday
 * (the week range is in the edit dialog). Same selection and events as the table.
 */
defineProps<{ rows: PaycheckLogRow[] }>();
const selectedIds = defineModel<string[]>('selectedIds', { required: true });
const emit = defineEmits<{
	edit: [log: PaycheckLogRow];
	email: [log: PaycheckLogRow];
	delete: [log: PaycheckLogRow];
}>();

const { t, locale } = useI18n();
</script>

<template>
	<v-list class="rounded" lines="two">
		<v-list-item v-for="log in rows" :key="log.id">
			<template #prepend>
				<v-checkbox-btn
					:model-value="selectedIds.includes(log.id)"
					:disabled="!canEmail(log)"
					:aria-label="canEmail(log) ? t('payments.select') : t('payments.onlyCompleted')"
					class="mr-2"
					@update:model-value="selectedIds = toggleId(selectedIds, log.id)"
				/>
			</template>
			<v-list-item-title>{{ log.firstName }} {{ log.lastName }}</v-list-item-title>
			<v-list-item-subtitle>
				{{ t('payment.payday') }} {{ formatMonthDay(log.payday, locale) }}
			</v-list-item-subtitle>
			<div class="d-flex align-center ga-2 mt-1">
				<strong>{{ formatCurrency(log.employeeCut) }}</strong>
				<span class="text-body-2 text-medium-emphasis">{{
					t(`payment.statuses.${log.status}`)
				}}</span>
				<v-chip
					v-if="log.kind === 'lump_sum'"
					size="x-small"
					variant="outlined"
					color="primary"
					>{{ t('payment.lumpSum') }}</v-chip
				>
			</div>
			<template #append>
				<PaycheckRowActions
					:log="log"
					vertical
					@edit="emit('edit', log)"
					@email="emit('email', log)"
					@delete="emit('delete', log)"
				/>
			</template>
		</v-list-item>
	</v-list>
</template>

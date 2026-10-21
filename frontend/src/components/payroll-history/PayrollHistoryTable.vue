<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { formatCurrency } from '../../utils/currency';
import { formatMonthDay } from '../../utils/week';
import PaycheckRowActions from './PaycheckRowActions.vue';
import { canEmail, toggleId, type PaycheckLogRow } from './types';

/**
 * Payroll History on a wide screen: one page of paycheck logs, with a checkbox column for
 * picking completed ones (the header box picks this page's). The selection lives in the view,
 * so it survives paging.
 */
const props = defineProps<{ rows: PaycheckLogRow[] }>();
const selectedIds = defineModel<string[]>('selectedIds', { required: true });
const emit = defineEmits<{
	edit: [log: PaycheckLogRow];
	email: [log: PaycheckLogRow];
	delete: [log: PaycheckLogRow];
}>();

const { t, locale } = useI18n();

const selectable = computed(() => props.rows.filter(canEmail));
const pageSelection = computed(() => {
	const picked = selectable.value.filter((log) => selectedIds.value.includes(log.id));
	if (picked.length === 0) return 'none';
	return picked.length === selectable.value.length ? 'all' : 'some';
});

function togglePage(): void {
	const pageIds = selectable.value.map((log) => log.id);
	selectedIds.value =
		pageSelection.value === 'all'
			? selectedIds.value.filter((id) => !pageIds.includes(id))
			: [...new Set([...selectedIds.value, ...pageIds])];
}
</script>

<template>
	<v-table density="compact" striped="odd">
		<thead>
			<tr>
				<th class="select-cell">
					<v-checkbox-btn
						:model-value="pageSelection === 'all'"
						:indeterminate="pageSelection === 'some'"
						:disabled="selectable.length === 0"
						:aria-label="t('payments.selectAll')"
						:title="t('payments.selectAll')"
						density="compact"
						@update:model-value="togglePage"
					/>
				</th>
				<th>{{ t('employees.firstName') }} {{ t('employees.lastName') }}</th>
				<th>{{ t('payment.weekStart') }}</th>
				<th>{{ t('payment.weekEnd') }}</th>
				<th>{{ t('payment.payday') }}</th>
				<th>{{ t('payment.status') }}</th>
				<th class="text-right">{{ t('payment.totalPay') }}</th>
				<th>{{ t('common.actions') }}</th>
			</tr>
		</thead>
		<tbody>
			<tr v-for="log in rows" :key="log.id">
				<td class="select-cell">
					<!-- Drafts can't be emailed, so they can't be picked either. -->
					<v-checkbox-btn
						:model-value="selectedIds.includes(log.id)"
						:disabled="!canEmail(log)"
						:aria-label="
							canEmail(log) ? t('payments.select') : t('payments.onlyCompleted')
						"
						:title="canEmail(log) ? undefined : t('payments.onlyCompleted')"
						density="compact"
						@update:model-value="selectedIds = toggleId(selectedIds, log.id)"
					/>
				</td>
				<td>{{ log.firstName }} {{ log.lastName }}</td>
				<td>{{ formatMonthDay(log.weekStart, locale) }}</td>
				<td>{{ formatMonthDay(log.weekEnd, locale) }}</td>
				<td>{{ formatMonthDay(log.payday, locale) }}</td>
				<td>
					{{ t(`payment.statuses.${log.status}`) }}
					<v-chip
						v-if="log.kind === 'lump_sum'"
						size="x-small"
						variant="outlined"
						color="primary"
						class="ml-1"
						>{{ t('payment.lumpSum') }}</v-chip
					>
				</td>
				<td class="text-right">{{ formatCurrency(log.employeeCut) }}</td>
				<td>
					<PaycheckRowActions
						:log="log"
						@edit="emit('edit', log)"
						@email="emit('email', log)"
						@delete="emit('delete', log)"
					/>
				</td>
			</tr>
		</tbody>
	</v-table>
</template>

<style scoped>
.select-cell {
	width: 48px;
	padding-inline-end: 0 !important;
}
</style>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { formatMonthDay } from '../../utils/week';
import { useToastStore } from '../../store/toast-store';
import { apiJson } from '../../utils/api';

type HistoryRow = {
	id: string;
	firstName: string;
	lastName: string;
	payday: string;
	createdAt: string;
	createdByEmail: string | null;
};

// Newest-processed first; enough to see recent activity without paging.
const MAX_ROWS = 10;

const { t, locale } = useI18n();
const emit = defineEmits<{ collapse: [] }>();
const rows = ref<HistoryRow[]>([]);
const loading = ref(false);
const toast = useToastStore();

async function refresh(): Promise<void> {
	loading.value = true;
	try {
		const logs = await apiJson<HistoryRow[]>('/api/paycheck-logs');
		rows.value = logs.sort((a, b) => b.createdAt.localeCompare(a.createdAt)).slice(0, MAX_ROWS);
	} catch (error) {
		toast.fromError(error);
	} finally {
		loading.value = false;
	}
}

onMounted(refresh);
defineExpose({ refresh });

const timeFormatter = computed(
	() => new Intl.DateTimeFormat(locale.value, { hour: 'numeric', minute: '2-digit' }),
);
</script>

<template>
	<v-card :title="t('history.title')" :loading="loading" flat class="history">
		<template #append>
			<v-btn
				icon="mdi-chevron-right"
				:aria-label="t('history.collapse')"
				variant="tonal"
				density="comfortable"
				@click="emit('collapse')"
			/>
		</template>
		<v-table v-if="rows.length" density="compact" class="history-table">
			<thead>
				<tr>
					<th>{{ t('history.employee') }}</th>
					<th>{{ t('history.processedAt') }}</th>
				</tr>
			</thead>
			<tbody>
				<tr v-for="row in rows" :key="row.id">
					<!-- Who processed it sits under the name, so the table fits a narrow column. -->
					<td class="py-1">
						<div class="font-weight-medium">{{ row.firstName }} {{ row.lastName }}</div>
						<div class="text-caption text-medium-emphasis by">
							{{
								t('history.processedByUser', {
									user: row.createdByEmail?.split('@')[0] ?? '—',
								})
							}}
						</div>
					</td>
					<td class="py-1">
						<div>{{ formatMonthDay(row.createdAt, locale) }}</div>
						<div class="text-caption text-medium-emphasis">
							{{ timeFormatter.format(new Date(row.createdAt)) }}
						</div>
					</td>
				</tr>
			</tbody>
		</v-table>
		<v-card-text v-else-if="!loading" class="text-medium-emphasis">
			{{ t('history.empty') }}
		</v-card-text>
	</v-card>
</template>

<style scoped>
/* Fills its container (the right gutter); only the table scrolls. */
.history {
	display: flex;
	flex-direction: column;
}

.history-table {
	flex: 1;
	min-height: 0;
	overflow-y: auto;
}

.history-table td,
.history-table th {
	white-space: nowrap;
	padding-inline: 12px !important;
}

.by {
	max-width: 14rem;
	overflow: hidden;
	text-overflow: ellipsis;
}
</style>

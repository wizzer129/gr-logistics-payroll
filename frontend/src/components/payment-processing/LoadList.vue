<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { useDisplay } from 'vuetify';
import type { Load } from '../../store/payment-processing-store';
import { formatCurrency } from '../../utils/currency';

const { t } = useI18n();
// A phone can't fit five columns: each load becomes a two-line list row instead.
const { xs } = useDisplay();
defineProps<{ loads: Load[] }>();
const emit = defineEmits<{ remove: [index: number] }>();
</script>

<template>
	<!-- Empty on a phone renders nothing, so it leaves no gap. -->
	<template v-if="xs">
		<v-list v-if="loads.length" class="rounded" lines="two">
			<v-list-item
				v-for="(load, index) in loads"
				:key="index"
				:title="load.storeName"
				:subtitle="`${load.loadDate.slice(0, 10)} · ${t('payment.pay')} ${formatCurrency(load.pay)} · ${t('payment.extraPay')} ${formatCurrency(load.extraPay ?? 0)}`"
			>
				<template #append>
					<v-btn
						icon="mdi-delete"
						size="small"
						variant="text"
						:aria-label="t('common.delete')"
						@click="emit('remove', index)"
					/>
				</template>
			</v-list-item>
		</v-list>
	</template>
	<v-table v-else density="comfortable" striped="odd">
		<thead>
			<tr>
				<th class="text-start">{{ t('payment.date') }}</th>
				<th class="text-start">{{ t('payment.storeName') }}</th>
				<th class="text-start">{{ t('payment.pay') }}</th>
				<th class="text-start">{{ t('payment.extraPay') }}</th>
				<th />
			</tr>
		</thead>
		<tbody>
			<tr v-for="(load, index) in loads" :key="index">
				<!-- loadDate is "YYYY-MM-DD" from both the date input and the API; slice is defensive. -->
				<td class="text-start">{{ load.loadDate.slice(0, 10) }}</td>
				<td class="text-start">{{ load.storeName }}</td>
				<td class="text-start">{{ formatCurrency(load.pay) }}</td>
				<td class="text-start">{{ formatCurrency(load.extraPay ?? 0) }}</td>
				<td class="text-start">
					<v-btn
						icon="mdi-delete"
						size="small"
						variant="text"
						:aria-label="t('common.delete')"
						@click="emit('remove', index)"
					/>
				</td>
			</tr>
		</tbody>
	</v-table>
</template>
<style scoped>
.v-table--density-comfortable {
	border-radius: 6px;
}
</style>

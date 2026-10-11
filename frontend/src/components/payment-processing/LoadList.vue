<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { Load } from '../../store/payment-processing-store';
import { formatCurrency } from '../../utils/currency';

const { t } = useI18n();
defineProps<{ loads: Load[] }>();
const emit = defineEmits<{ remove: [index: number] }>();
</script>

<template>
	<v-table density="compact" striped="odd">
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
				<!-- loadDate is "YYYY-MM-DD" from a date input when added locally, but a full
				ISO datetime when it comes back from the API (edit flow) — truncate either way. -->
				<td class="text-start">{{ load.loadDate.slice(0, 10) }}</td>
				<td class="text-start">{{ load.storeName }}</td>
				<td class="text-start">{{ formatCurrency(load.pay) }}</td>
				<td class="text-start">{{ formatCurrency(load.extraPay ?? 0) }}</td>
				<td class="text-start">
					<v-btn
						icon="mdi-delete"
						size="small"
						variant="text"
						@click="emit('remove', index)"
					/>
				</td>
			</tr>
		</tbody>
	</v-table>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { Load } from '../../store/payment-processing-store';

const { t, n } = useI18n();
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
				<td class="text-start">{{ load.loadDate }}</td>
				<td class="text-start">{{ load.storeName }}</td>
				<td class="text-start">{{ n(load.pay, 'currency') }}</td>
				<td class="text-start">{{ n(load.extraPay ?? 0, 'currency') }}</td>
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

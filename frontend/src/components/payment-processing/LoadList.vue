<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { Load } from '../../store/payment-processing-store';

const { t } = useI18n();
defineProps<{ loads: Load[] }>();
const emit = defineEmits<{ remove: [index: number] }>();
</script>

<template>
	<v-table>
		<thead>
			<tr>
				<th>{{ t('payment.date') }}</th>
				<th>{{ t('payment.storeName') }}</th>
				<th>{{ t('payment.pay') }}</th>
				<th>{{ t('payment.extraPay') }}</th>
				<th />
			</tr>
		</thead>
		<tbody>
			<tr v-for="(load, index) in loads" :key="index">
				<td>{{ load.loadDate }}</td>
				<td>{{ load.storeName }}</td>
				<td>{{ load.pay }}</td>
				<td>{{ load.extraPay }}</td>
				<td>
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

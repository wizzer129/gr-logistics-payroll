<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useToastStore } from '../../store/toast-store';
import { apiJson } from '../../utils/api';
import { currentMonth, previousMonth } from '../../utils/month';
import type { RecipientSelection } from '../../utils/recipients';
import RecipientSelect from './RecipientSelect.vue';

/**
 * Sends a month's report now (EMAIL-SPEC.md §8): to resend one, or for a month the automatic
 * send didn't cover. With nobody picked it goes to the monthly report recipients.
 */
const emit = defineEmits<{ queued: [] }>();

const { t } = useI18n();
const toast = useToastStore();

const month = ref(previousMonth());
const recipients = ref<RecipientSelection>({ userIds: [], employeeIds: [] });
const sending = ref(false);
const maxMonth = currentMonth();

async function send(): Promise<void> {
	sending.value = true;
	try {
		const picked = recipients.value.userIds;
		const { outboxIds } = await apiJson<{ outboxIds: string[] }>('/api/email/monthly-report', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({
				month: month.value,
				...(picked.length > 0 ? { userIds: picked } : {}),
			}),
		});
		toast.success(t('monthlyReport.queued', { count: outboxIds.length }, outboxIds.length));
		recipients.value = { userIds: [], employeeIds: [] };
		emit('queued');
	} catch (error) {
		toast.fromError(error);
	} finally {
		sending.value = false;
	}
}
</script>

<template>
	<v-card :title="t('monthlyReport.sendTitle')">
		<v-card-text>
			<p class="mb-4 text-medium-emphasis">{{ t('monthlyReport.sendHint') }}</p>
			<v-text-field
				v-model="month"
				type="month"
				:label="t('monthlyReport.month')"
				:max="maxMonth"
			/>
			<RecipientSelect
				v-model="recipients"
				:label="t('monthlyReport.recipients')"
				:hint="t('monthlyReport.recipientsHint')"
				persistent-hint
				users-only
			/>
		</v-card-text>
		<v-card-actions>
			<v-spacer />
			<v-btn
				color="primary"
				variant="flat"
				prepend-icon="mdi-email-fast-outline"
				:loading="sending"
				:disabled="sending || !month"
				@click="send"
				>{{ t('monthlyReport.send') }}</v-btn
			>
		</v-card-actions>
	</v-card>
</template>

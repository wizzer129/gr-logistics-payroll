<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useUnsavedChanges } from '../composables/useUnsavedChanges';
import { useToastStore } from '../store/toast-store';
import { apiJson } from '../utils/api';
import type { RecipientSelection } from '../utils/recipients';
import RecipientSelect from '../components/email/RecipientSelect.vue';

const { t } = useI18n();
const toast = useToastStore();

// Monthly report recipients: users only (the report shows everyone's pay).
const recipients = ref<RecipientSelection>({ userIds: [], employeeIds: [] });
const saved = ref<string[]>([]);
const loading = ref(false);
const saving = ref(false);

const isDirty = () =>
	[...recipients.value.userIds].sort().join() !== [...saved.value].sort().join();
useUnsavedChanges().guardLeave(isDirty);

onMounted(async () => {
	loading.value = true;
	try {
		const { userIds } = await apiJson<{ userIds: string[] }>(
			'/api/email/monthly-report/recipients',
		);
		recipients.value = { userIds, employeeIds: [] };
		saved.value = userIds;
	} catch (error) {
		toast.fromError(error);
	} finally {
		loading.value = false;
	}
});

async function save(): Promise<void> {
	saving.value = true;
	try {
		const { userIds } = await apiJson<{ userIds: string[] }>(
			'/api/email/monthly-report/recipients',
			{
				method: 'PUT',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ userIds: recipients.value.userIds }),
			},
		);
		recipients.value = { userIds, employeeIds: [] };
		saved.value = userIds;
		toast.success(t('settings.monthlySaved'));
	} catch (error) {
		toast.fromError(error);
	} finally {
		saving.value = false;
	}
}
</script>

<template>
	<v-container>
		<h1 class="text-h5 mb-4">{{ t('nav.settings') }}</h1>

		<v-card :title="t('settings.monthlyTitle')" :loading="loading" max-width="640">
			<v-card-text>
				<p class="mb-4 text-medium-emphasis">{{ t('settings.monthlyHint') }}</p>
				<RecipientSelect
					v-model="recipients"
					:label="t('settings.monthlyRecipients')"
					users-only
				/>
			</v-card-text>
			<v-card-actions>
				<v-spacer />
				<v-btn
					color="primary"
					variant="flat"
					:loading="saving"
					:disabled="saving || !isDirty()"
					@click="save"
					>{{ t('common.save') }}</v-btn
				>
			</v-card-actions>
		</v-card>
	</v-container>
</template>

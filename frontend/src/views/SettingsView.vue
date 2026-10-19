<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useUnsavedChanges } from '../composables/useUnsavedChanges';
import { useToastStore } from '../store/toast-store';
import { apiJson } from '../utils/api';
import type { RecipientSelection } from '../utils/recipients';
import MonthlyReportSendCard from '../components/email/MonthlyReportSendCard.vue';
import RecipientSelect from '../components/email/RecipientSelect.vue';
import SentEmailsCard from '../components/email/SentEmailsCard.vue';
import ThemeSettingsCard from '../components/settings/ThemeSettingsCard.vue';

const { t } = useI18n();
const toast = useToastStore();

// Monthly report recipients: users only (the report shows everyone's pay).
const recipients = ref<RecipientSelection>({ userIds: [], employeeIds: [] });
const saved = ref<string[]>([]);
const loading = ref(false);
const saving = ref(false);
const sentEmails = ref<InstanceType<typeof SentEmailsCard> | null>(null);

const isDirty = () =>
	[...recipients.value.userIds].sort().join() !== [...saved.value].sort().join();
// A theme picked in the preview but not applied counts as unsaved too.
const themeCard = ref<InstanceType<typeof ThemeSettingsCard> | null>(null);
useUnsavedChanges().guardLeave(() => isDirty() || Boolean(themeCard.value?.isDirty));

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

		<v-row>
			<v-col cols="12">
				<ThemeSettingsCard ref="themeCard" />
			</v-col>
			<v-col cols="12" md="6">
				<v-card :title="t('settings.monthlyTitle')" :loading="loading" class="h-100">
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
			</v-col>
			<v-col cols="12" md="6">
				<MonthlyReportSendCard class="h-100" @queued="sentEmails?.refresh()" />
			</v-col>
			<v-col cols="12">
				<SentEmailsCard ref="sentEmails" />
			</v-col>
		</v-row>
	</v-container>
</template>

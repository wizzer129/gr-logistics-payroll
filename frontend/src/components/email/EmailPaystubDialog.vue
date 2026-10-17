<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useToastStore } from '../../store/toast-store';
import { ApiError, apiJson } from '../../utils/api';
import type { RecipientSelection } from '../../utils/recipients';
import RecipientSelect from './RecipientSelect.vue';

/**
 * Emails a completed paycheck log's pay stub (PDF attached) to picked users/employees, each
 * in their own language (EMAIL-SPEC.md §8). The paycheck's own employee starts selected
 * when they have an email.
 */
const open = defineModel<boolean>({ required: true });
const props = defineProps<{ logId: string | null; employeeId: string | null }>();

const { t } = useI18n();
const toast = useToastStore();

const recipients = ref<RecipientSelection>({ userIds: [], employeeIds: [] });
const sending = ref(false);
const error = ref<string | null>(null);
const count = computed(() => recipients.value.userIds.length + recipients.value.employeeIds.length);

watch(open, (isOpen) => {
	if (!isOpen) return;
	recipients.value = { userIds: [], employeeIds: [] };
	error.value = null;
});

async function send(): Promise<void> {
	if (!props.logId) return;
	sending.value = true;
	error.value = null;
	try {
		const { outboxIds } = await apiJson<{ outboxIds: string[] }>(
			`/api/paycheck-logs/${props.logId}/email`,
			{
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify(recipients.value),
			},
		);
		toast.success(t('emailPaystub.queued', { count: outboxIds.length }, outboxIds.length));
		open.value = false;
	} catch (e) {
		// Shown in the dialog, which stays open so the picks aren't lost.
		if (!(e instanceof ApiError)) throw e;
		error.value = e.message;
	} finally {
		sending.value = false;
	}
}
</script>

<template>
	<v-dialog v-model="open" max-width="520" :persistent="sending">
		<v-card :title="t('emailPaystub.title')">
			<v-card-text>
				<p class="mb-4 text-medium-emphasis">{{ t('emailPaystub.intro') }}</p>
				<v-alert v-if="error" type="error" density="compact" class="mb-4">{{
					error
				}}</v-alert>
				<RecipientSelect
					v-if="open"
					v-model="recipients"
					:label="t('emailPaystub.recipients')"
					:preselect-employee-id="employeeId ?? undefined"
				/>
			</v-card-text>
			<v-card-actions>
				<v-spacer />
				<v-btn variant="outlined" :disabled="sending" @click="open = false">{{
					t('common.cancel')
				}}</v-btn>
				<v-btn
					color="primary"
					variant="flat"
					prepend-icon="mdi-email-fast-outline"
					:loading="sending"
					:disabled="sending || count === 0"
					@click="send"
					>{{ t('emailPaystub.send') }}</v-btn
				>
			</v-card-actions>
		</v-card>
	</v-dialog>
</template>

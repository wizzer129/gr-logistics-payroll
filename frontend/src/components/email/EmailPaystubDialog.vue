<script setup lang="ts">
import { computed, ref, useTemplateRef, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useToastStore } from '../../store/toast-store';
import { ApiError, apiJson } from '../../utils/api';
import { emptySelection, recipientCount, type RecipientSelection } from '../../utils/recipients';
import RecipientSelect from './RecipientSelect.vue';

/**
 * Emails completed paycheck logs' pay stubs (PDFs attached) to picked users/employees, each
 * in their own language, and to typed-in addresses (EMAIL-SPEC.md §8). One log: its own
 * employee starts selected. Several: they go out in one email per recipient, and an employee
 * is only offered when every log is theirs, so nobody gets someone else's pay.
 */
export type EmailableLog = { id: string; employeeId: string };

const open = defineModel<boolean>({ required: true });
const props = defineProps<{ logs: EmailableLog[] }>();

const { t } = useI18n();
const toast = useToastStore();

const recipients = ref<RecipientSelection>(emptySelection());
const recipientSelect = useTemplateRef<InstanceType<typeof RecipientSelect>>('recipientSelect');
const sending = ref(false);
const error = ref<string | null>(null);
const count = computed(() => recipientCount(recipients.value));
// Somebody to send to, and no mistyped address among them.
const canSend = computed(
	() => count.value > 0 && recipientSelect.value?.isValid !== false && !sending.value,
);
const bulk = computed(() => props.logs.length > 1);

const ownersOfLogs = computed(() => [...new Set(props.logs.map((log) => log.employeeId))]);
// One log: anyone. Several: their one employee, if they're all the same one's; else nobody.
const offeredEmployees = computed(() =>
	bulk.value ? (ownersOfLogs.value.length === 1 ? ownersOfLogs.value : []) : undefined,
);

watch(open, (isOpen) => {
	if (!isOpen) return;
	recipients.value = emptySelection();
	error.value = null;
});

async function send(): Promise<void> {
	if (props.logs.length === 0) return;
	sending.value = true;
	error.value = null;
	try {
		const [url, body] = bulk.value
			? ['/api/email/paystubs', { paycheckLogIds: props.logs.map((log) => log.id) }]
			: [`/api/paycheck-logs/${props.logs[0].id}/email`, {}];
		const { outboxIds } = await apiJson<{ outboxIds: string[] }>(url, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ ...body, ...recipients.value }),
		});
		toast.success(
			t(
				bulk.value ? 'emailPaystub.queuedMany' : 'emailPaystub.queued',
				{ count: outboxIds.length, stubs: props.logs.length },
				outboxIds.length,
			),
		);
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
	<v-dialog v-model="open" max-width="560" :persistent="sending">
		<v-card
			:title="
				bulk ? t('emailPaystub.titleMany', { count: logs.length }) : t('emailPaystub.title')
			"
		>
			<v-card-text>
				<p class="mb-4 text-medium-emphasis">
					{{ bulk ? t('emailPaystub.introMany') : t('emailPaystub.intro') }}
				</p>
				<v-alert v-if="error" type="error" density="compact" class="mb-4">{{
					error
				}}</v-alert>
				<RecipientSelect
					v-if="open"
					ref="recipientSelect"
					v-model="recipients"
					:label="t('emailPaystub.recipients')"
					:employee-ids="offeredEmployees"
					:preselect-employee-id="bulk ? undefined : logs[0]?.employeeId"
					allow-typed
				/>
				<p
					v-if="bulk && ownersOfLogs.length > 1"
					class="mt-2 text-caption text-medium-emphasis"
				>
					{{ t('emailPaystub.employeesHidden') }}
				</p>
			</v-card-text>
			<v-card-actions>
				<v-spacer />
				<v-btn variant="text" :disabled="sending" @click="open = false">{{
					t('common.cancel')
				}}</v-btn>
				<v-btn
					color="primary"
					variant="flat"
					prepend-icon="mdi-email-fast-outline"
					:loading="sending"
					:disabled="!canSend"
					@click="send"
					>{{ t('emailPaystub.send') }}</v-btn
				>
			</v-card-actions>
		</v-card>
	</v-dialog>
</template>

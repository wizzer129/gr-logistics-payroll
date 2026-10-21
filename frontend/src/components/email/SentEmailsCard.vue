<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useDisplay } from 'vuetify';
import { useToastStore } from '../../store/toast-store';
import { apiJson, apiRequest } from '../../utils/api';

/**
 * The most recent emails and how each one went (EMAIL-SPEC.md §5), with a retry for failed
 * ones. A failed password reset can't be retried: its link was scrubbed, so the user asks for
 * a new one.
 */
type OutboxEntry = {
	id: string;
	kind: 'test_email' | 'password_reset' | 'paystub' | 'paystubs' | 'monthly_report';
	recipient: string;
	status: 'pending' | 'sent' | 'failed';
	attempts: number;
	subject: string | null;
	lastError: string | null;
	createdAt: string;
	sentAt: string | null;
	nextAttemptAt: string;
	retryable: boolean;
};

const { t, locale } = useI18n();
const toast = useToastStore();
// A phone can't fit the table: each email becomes a stacked list row instead.
const { xs } = useDisplay();

const emails = ref<OutboxEntry[]>([]);
const loading = ref(false);
const retrying = ref<string | null>(null);

const dateTime = computed(
	() => new Intl.DateTimeFormat(locale.value, { dateStyle: 'medium', timeStyle: 'short' }),
);
const formatTime = (iso: string) => dateTime.value.format(new Date(iso));

const statusColor = { pending: 'info', sent: 'success', failed: 'error' } as const;

async function refresh(): Promise<void> {
	loading.value = true;
	try {
		emails.value = await apiJson<OutboxEntry[]>('/api/email/outbox');
	} catch (error) {
		toast.fromError(error);
	} finally {
		loading.value = false;
	}
}

async function retry(email: OutboxEntry): Promise<void> {
	retrying.value = email.id;
	try {
		await apiRequest(`/api/email/outbox/${email.id}/retry`, { method: 'POST' });
		toast.success(t('sentEmails.retried'));
		await refresh();
	} catch (error) {
		toast.fromError(error);
	} finally {
		retrying.value = null;
	}
}

onMounted(refresh);
defineExpose({ refresh });
</script>

<template>
	<v-card :title="t('sentEmails.title')" :subtitle="t('sentEmails.hint')" :loading="loading">
		<template #append>
			<v-btn
				icon="mdi-refresh"
				variant="text"
				:aria-label="t('sentEmails.refresh')"
				:disabled="loading"
				@click="refresh"
			/>
		</template>
		<v-card-text>
			<p v-if="!loading && emails.length === 0" class="text-medium-emphasis">
				{{ t('sentEmails.empty') }}
			</p>
			<v-list v-else-if="xs" class="mx-n4">
				<v-list-item v-for="email in emails" :key="email.id">
					<v-list-item-title class="d-flex align-center ga-2">
						{{ t(`sentEmails.kinds.${email.kind}`) }}
						<v-chip :color="statusColor[email.status]" size="x-small" label>
							{{ t(`sentEmails.statuses.${email.status}`) }}
						</v-chip>
					</v-list-item-title>
					<v-list-item-subtitle>{{ email.recipient }}</v-list-item-subtitle>
					<div class="text-caption text-medium-emphasis mt-1">
						{{ formatTime(email.sentAt ?? email.createdAt) }}
					</div>
					<div v-if="email.subject" class="text-caption text-medium-emphasis">
						{{ email.subject }}
					</div>
					<div
						v-if="email.status === 'pending' && email.attempts > 0"
						class="text-caption text-medium-emphasis"
					>
						{{ t('sentEmails.nextAttempt', { time: formatTime(email.nextAttemptAt) }) }}
					</div>
					<div v-if="email.lastError" class="text-caption text-error">
						{{ email.lastError }}
					</div>
					<v-btn
						v-if="email.retryable"
						class="mt-2"
						size="small"
						variant="tonal"
						color="primary"
						prepend-icon="mdi-email-sync-outline"
						:loading="retrying === email.id"
						:disabled="retrying !== null"
						@click="retry(email)"
						>{{ t('sentEmails.retry') }}</v-btn
					>
				</v-list-item>
			</v-list>
			<v-table v-else density="compact">
				<thead>
					<tr>
						<th>{{ t('sentEmails.when') }}</th>
						<th>{{ t('sentEmails.kind') }}</th>
						<th>{{ t('sentEmails.recipient') }}</th>
						<th>{{ t('sentEmails.status') }}</th>
						<th />
					</tr>
				</thead>
				<tbody>
					<tr v-for="email in emails" :key="email.id">
						<td class="text-no-wrap">
							{{ formatTime(email.sentAt ?? email.createdAt) }}
						</td>
						<td>
							<div>{{ t(`sentEmails.kinds.${email.kind}`) }}</div>
							<div v-if="email.subject" class="text-caption text-medium-emphasis">
								{{ email.subject }}
							</div>
						</td>
						<td>{{ email.recipient }}</td>
						<td>
							<v-chip :color="statusColor[email.status]" size="small" label>
								{{ t(`sentEmails.statuses.${email.status}`) }}
							</v-chip>
							<div
								v-if="email.status === 'pending' && email.attempts > 0"
								class="text-caption text-medium-emphasis"
							>
								{{
									t('sentEmails.nextAttempt', {
										time: formatTime(email.nextAttemptAt),
									})
								}}
							</div>
							<div v-if="email.lastError" class="text-caption text-error">
								{{ email.lastError }}
							</div>
						</td>
						<td class="text-right">
							<v-btn
								v-if="email.retryable"
								size="small"
								variant="tonal"
								color="primary"
								prepend-icon="mdi-email-sync-outline"
								:loading="retrying === email.id"
								:disabled="retrying !== null"
								@click="retry(email)"
								>{{ t('sentEmails.retry') }}</v-btn
							>
						</td>
					</tr>
				</tbody>
			</v-table>
		</v-card-text>
	</v-card>
</template>

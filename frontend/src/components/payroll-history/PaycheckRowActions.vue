<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { canEmail, type PaycheckLogRow } from './types';

/** A row's edit / email / delete buttons, in a row (table) or a column (phone list). */
defineProps<{ log: PaycheckLogRow; vertical?: boolean }>();
const emit = defineEmits<{ edit: []; email: []; delete: [] }>();

const { t } = useI18n();
</script>

<template>
	<div :class="vertical ? 'd-flex flex-column' : 'text-no-wrap'">
		<v-btn
			icon="mdi-pencil"
			size="small"
			variant="text"
			:aria-label="t('common.edit')"
			@click="emit('edit')"
		/>
		<v-btn
			v-if="canEmail(log)"
			icon="mdi-email-outline"
			size="small"
			variant="text"
			:aria-label="t('emailPaystub.title')"
			@click="emit('email')"
		/>
		<v-btn
			icon="mdi-delete"
			size="small"
			variant="text"
			:aria-label="t('common.delete')"
			@click="emit('delete')"
		/>
	</div>
</template>

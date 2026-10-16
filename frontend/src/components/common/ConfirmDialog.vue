<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useConfirmStore, type ConfirmOptions } from '../../store/confirm-store';

const { t } = useI18n();
const store = useConfirmStore();

const open = computed({
	get: () => store.pending !== null,
	set: (value) => {
		if (!value) store.answer(false);
	},
});

// Kept after answering, so the text doesn't blank out during the closing transition.
const shown = ref<ConfirmOptions | null>(null);
watch(
	() => store.pending,
	(pending) => {
		if (pending) shown.value = pending;
	},
);
</script>

<template>
	<v-dialog v-model="open" max-width="440">
		<v-card v-if="shown" :title="shown.title" :text="shown.text">
			<v-card-actions>
				<v-spacer />
				<v-btn variant="text" @click="store.answer(false)">{{ t('common.cancel') }}</v-btn>
				<v-btn
					:color="shown.color ?? 'primary'"
					variant="flat"
					@click="store.answer(true)"
					>{{ shown.confirmText ?? t('common.confirm') }}</v-btn
				>
			</v-card-actions>
		</v-card>
	</v-dialog>
</template>

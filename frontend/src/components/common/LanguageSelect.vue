<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';

// `en` / `es`, shown by name. Other attributes (label, hint…) pass through to the v-select.
const value = defineModel<'en' | 'es'>({ required: true });
const { t } = useI18n();
const items = computed(() => [
	{ title: t('common.languages.en'), value: 'en' },
	{ title: t('common.languages.es'), value: 'es' },
]);
</script>

<template>
	<v-select v-model="value" :items="items" />
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { currentMode, toggleTheme } from '../../plugins/vuetify';

const { t } = useI18n();

const isDark = computed(() => currentMode() === 'dark');
</script>

<template>
	<v-btn
		:icon="isDark ? 'mdi-weather-sunny' : 'mdi-weather-night'"
		:aria-label="t('nav.toggleTheme')"
		variant="text"
		@click="toggleTheme"
	/>
</template>

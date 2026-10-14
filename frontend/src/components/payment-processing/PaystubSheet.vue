<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import Paystub, { type PaystubData } from './Paystub.vue';

defineProps<{ stub: PaystubData }>();

const { t } = useI18n();
</script>

<template>
	<div class="paystub-sheet">
		<Paystub v-bind="stub" :copy-label="t('paystub.contractor')" />
		<div class="cut-line" aria-hidden="true" />
		<Paystub v-bind="stub" :copy-label="t('paystub.ownerOperator')" />
	</div>
</template>

<style scoped>
/* Two copies stacked for one letter page, with a dashed line to cut along. */
.paystub-sheet {
	display: flex;
	flex-direction: column;
	align-items: center;
}

.paystub-sheet :deep(.paystub) {
	break-inside: avoid;
}

.cut-line {
	width: 7.5in;
	max-width: 100%;
	margin: 0.18in 0;
	border-top: 1px dashed #9aa4b1;
}
</style>

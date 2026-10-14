<script setup lang="ts">
import { nextTick, ref } from 'vue';
import type { PaystubData } from './Paystub.vue';
import PaystubSheet from './PaystubSheet.vue';

/**
 * Prints the two-copy paystub sheet on its own. The sheet is only mounted (outside the app,
 * straight under <body>) while printing, and the <html> class hides everything else, so the
 * page holds just the two paystubs. Call `print(stub)` through a template ref.
 */
const printing = ref<PaystubData | null>(null);

async function print(stub: PaystubData): Promise<void> {
	printing.value = stub;
	await nextTick();
	document.documentElement.classList.add('paystub-printing');

	// Browsers name a "Save as PDF" file after the page title, e.g. "Mauricio Sanchez - 2026-10-16".
	const originalTitle = document.title;
	document.title = `${stub.employeeName} - ${stub.payday}`;

	window.addEventListener(
		'afterprint',
		() => {
			document.title = originalTitle;
			document.documentElement.classList.remove('paystub-printing');
			printing.value = null;
		},
		{ once: true },
	);
	window.print();
}

defineExpose({ print });
</script>

<template>
	<Teleport to="body">
		<div v-if="printing" class="paystub-print-sheet">
			<PaystubSheet :stub="printing" />
		</div>
	</Teleport>
</template>

<style>
/* Unscoped: the paystub print sheet is teleported to <body>, outside this component. */
.paystub-print-sheet {
	display: none;
}

@media print {
	/* An open dialog makes Vuetify pin <html> at a negative offset (scroll lock), which
	   would push the sheet off the top of the page. */
	html.paystub-printing {
		position: static !important;
		top: auto !important;
		left: auto !important;
		overflow: visible !important;
	}

	html.paystub-printing body > :not(.paystub-print-sheet) {
		display: none !important;
	}

	html.paystub-printing .paystub-print-sheet {
		display: block;
	}
}

@page {
	size: letter;
	margin: 0.4in;
}
</style>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import {
	checkCsvTotals,
	CsvLoadsError,
	parseLoadsCsv,
	type CsvTotalCheck,
	type ParsedLoadsCsv,
} from '../../utils/csv-loads';
import { formatCurrency } from '../../utils/currency';

const { t } = useI18n();
// The selected employee's rate — uploading selects the file's employee, so this is theirs.
const props = defineProps<{ payRatePercent: number }>();
const emit = defineEmits<{ processed: [csv: ParsedLoadsCsv]; cleared: [] }>();

// v-file-input's model is a File (single) or File[] depending on the Vuetify version.
const file = ref<File | File[] | null>(null);
const parsed = ref<ParsedLoadsCsv | null>(null);
const error = ref<string | null>(null);

const selectedFile = computed(() =>
	Array.isArray(file.value) ? (file.value[0] ?? null) : file.value,
);

// Picking a file applies it right away; removing it takes its loads back out.
watch(selectedFile, async (csvFile) => {
	const hadLoads = parsed.value !== null;
	parsed.value = null;
	error.value = null;
	if (!csvFile) {
		if (hadLoads) emit('cleared');
		return;
	}

	try {
		parsed.value = parseLoadsCsv(await csvFile.text());
		emit('processed', parsed.value);
	} catch (e) {
		if (!(e instanceof CsvLoadsError)) throw e;
		const reason = t(`payment.csvErrors.${e.code}`);
		error.value = e.line ? t('payment.csvErrorAtLine', { line: e.line, reason }) : reason;
	}
});

const totalsCheck = computed(() =>
	parsed.value ? checkCsvTotals(parsed.value, props.payRatePercent) : null,
);

function mismatchError(check: CsvTotalCheck | undefined): string[] {
	return check?.matches === false
		? [t('payment.csvMismatch', { expected: formatCurrency(check.expected) })]
		: [];
}

function display(amount: number | null | undefined): string {
	return amount === null || amount === undefined ? '' : formatCurrency(amount);
}

function onDelete(): void {
	file.value = null;
}
</script>

<template>
	<div>
		<v-file-input
			v-model="file"
			:label="t('payment.uploadCsv')"
			accept=".csv,text/csv"
			prepend-icon="mdi-file-delimited"
			:clearable="false"
		/>
		<v-alert v-if="error" type="error" density="compact" class="mb-4">{{ error }}</v-alert>
		<v-row dense>
			<v-col cols="12" sm="6">
				<v-text-field
					:model-value="display(parsed?.csvTotal)"
					:label="t('payment.csvTotal')"
					:error-messages="mismatchError(totalsCheck?.total)"
					:placeholder="parsed ? t('payment.csvNotInFile') : undefined"
					persistent-placeholder
					readonly
				/>
			</v-col>
			<v-col cols="12" sm="6">
				<v-text-field
					:model-value="display(parsed?.csvTotalPay)"
					:label="t('payment.csvTotalPay')"
					:error-messages="mismatchError(totalsCheck?.totalPay)"
					:placeholder="parsed ? t('payment.csvNotInFile') : undefined"
					persistent-placeholder
					readonly
				/>
			</v-col>
		</v-row>
		<v-btn
			v-if="selectedFile"
			color="error"
			variant="tonal"
			prepend-icon="mdi-delete"
			@click="onDelete"
		>
			{{ t('payment.deleteCsv') }}
		</v-btn>
	</div>
</template>

<script lang="ts">
import type { Load } from '../../store/payment-processing-store';

export type PaystubData = {
	employeeName: string;
	weekStart: string;
	weekEnd: string;
	payday: string;
	loads: Load[];
	summaryTotal: number;
	employeeCut: number;
	roundedPay: number;
	payRatePercent: number;
	/** A lump-sum payment: no loads, paid in full. */
	lumpSum?: boolean;
};
</script>

<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { formatCurrency } from '../../utils/currency';
import { parseLocalDate } from '../../utils/week';

/**
 * The document we export: a self-contained "paper" that keeps its own light palette in either
 * app theme, sized so two copies fit on one letter page (each stays under ~5in tall even
 * with 20+ loads, by splitting the loads into two side-by-side columns).
 */
const props = defineProps<
	PaystubData & {
		/** Which copy this is, e.g. "Contractor" or "Owner Operator". */
		copyLabel?: string;
	}
>();

const { t, locale } = useI18n();

// Above this many loads the table splits into two columns to keep the stub half-page tall.
const SINGLE_COLUMN_MAX = 8;

const sortedLoads = computed(() =>
	[...props.loads].sort((a, b) => a.loadDate.localeCompare(b.loadDate)),
);

const loadColumns = computed(() => {
	const loads = sortedLoads.value;
	if (loads.length <= SINGLE_COLUMN_MAX) return [loads];
	const half = Math.ceil(loads.length / 2);
	return [loads.slice(0, half), loads.slice(half)];
});

const extraPayTotal = computed(() => props.loads.reduce((sum, l) => sum + (l.extraPay ?? 0), 0));

function formatDate(iso: string, options: Intl.DateTimeFormatOptions): string {
	return new Intl.DateTimeFormat(locale.value, options).format(parseLocalDate(iso.slice(0, 10)));
}

const weekLabel = computed(() => {
	const short: Intl.DateTimeFormatOptions = { month: 'short', day: 'numeric' };
	return `${formatDate(props.weekStart, short)} – ${formatDate(props.weekEnd, { ...short, year: 'numeric' })}`;
});
const paydayLabel = computed(() =>
	formatDate(props.payday, { month: 'short', day: 'numeric', year: 'numeric' }),
);

function loadDateLabel(iso: string): string {
	return formatDate(iso, { weekday: 'short', month: 'numeric', day: 'numeric' });
}
</script>

<template>
	<article class="paystub">
		<header class="paystub-header">
			<div>
				<div class="company">{{ t('app.title') }}</div>
				<h2 class="employee">{{ employeeName }}</h2>
			</div>
			<div class="doc-title">
				<div>{{ t('paystub.title') }}</div>
				<div v-if="copyLabel" class="copy-label">{{ copyLabel }}</div>
			</div>
		</header>

		<dl class="meta">
			<div>
				<dt>{{ t('paystub.workWeek') }}</dt>
				<dd>{{ weekLabel }}</dd>
			</div>
			<div>
				<dt>{{ t('payment.payday') }}</dt>
				<dd>{{ paydayLabel }}</dd>
			</div>
			<div v-if="lumpSum">
				<dt>{{ t('paystub.paymentType') }}</dt>
				<dd>{{ t('payment.lumpSum') }}</dd>
			</div>
			<div v-else>
				<dt>{{ t('paystub.loadCount') }}</dt>
				<dd>{{ loads.length }}</dd>
			</div>
		</dl>

		<!-- A lump sum has no loads to list; its totals below say it all. -->
		<section v-if="!lumpSum" class="loads" :class="{ split: loadColumns.length > 1 }">
			<table v-for="(column, columnIndex) in loadColumns" :key="columnIndex">
				<thead>
					<tr>
						<th>{{ t('payment.date') }}</th>
						<th>{{ t('paystub.load') }}</th>
						<th class="num">{{ t('payment.pay') }}</th>
						<th class="num">{{ t('paystub.extra') }}</th>
					</tr>
				</thead>
				<tbody>
					<tr v-for="(load, index) in column" :key="index">
						<td class="date">{{ loadDateLabel(load.loadDate) }}</td>
						<td class="store">{{ load.storeName }}</td>
						<td class="num">{{ formatCurrency(load.pay) }}</td>
						<td class="num" :class="{ muted: !load.extraPay }">
							{{ load.extraPay ? formatCurrency(load.extraPay) : '—' }}
						</td>
					</tr>
				</tbody>
			</table>
			<p v-if="loads.length === 0" class="empty">{{ t('paystub.noLoads') }}</p>
		</section>

		<footer class="totals">
			<div class="breakdown">
				<div v-if="!lumpSum">
					<span>{{ t('paystub.extraPayTotal') }}</span>
					<strong>{{ formatCurrency(extraPayTotal) }}</strong>
				</div>
				<div>
					<span>{{ t('payment.summaryTotal') }}</span>
					<strong>{{ formatCurrency(summaryTotal) }}</strong>
				</div>
				<div>
					<span>{{ t('payment.employeeCut', { rate: payRatePercent }) }}</span>
					<strong>{{ formatCurrency(employeeCut) }}</strong>
				</div>
			</div>
			<div class="net">
				<span>{{ t('payment.totalPay') }}</span>
				<strong>{{ formatCurrency(roundedPay) }}</strong>
			</div>
		</footer>
	</article>
</template>

<style scoped>
/* Fixed light palette on purpose: this is a printable document, not themed UI. */
.paystub {
	--ink: #1c2430;
	--muted: #6b7686;
	--rule: #dfe4ea;
	--band: #f4f6f9;
	--accent: #1976d2;
	--accent-ink: #0d4a8f;

	width: 7.5in;
	max-width: 100%;
	box-sizing: border-box;
	padding: 0.28in 0.32in;
	background: #fff;
	color: var(--ink);
	font-family: 'Roboto', 'Helvetica Neue', Arial, sans-serif;
	font-size: 10.5px;
	line-height: 1.35;
	border-top: 4px solid var(--accent);
	/* Keep the bands and the Total Pay box when printed. */
	-webkit-print-color-adjust: exact;
	print-color-adjust: exact;
}

.paystub-header {
	display: flex;
	justify-content: space-between;
	align-items: flex-end;
	gap: 16px;
}

.company {
	font-size: 10px;
	font-weight: 600;
	letter-spacing: 0.12em;
	text-transform: uppercase;
	color: var(--accent-ink);
}

.employee {
	margin: 2px 0 0;
	font-size: 19px;
	font-weight: 600;
	line-height: 1.2;
}

.doc-title {
	display: flex;
	flex-direction: column;
	align-items: flex-end;
	gap: 3px;
	font-size: 11px;
	font-weight: 600;
	letter-spacing: 0.14em;
	text-transform: uppercase;
	color: var(--muted);
}

.copy-label {
	padding: 2px 8px;
	border: 1px solid var(--accent);
	border-radius: 999px;
	font-size: 9px;
	letter-spacing: 0.1em;
	color: var(--accent-ink);
}

.meta {
	display: grid;
	grid-template-columns: repeat(3, 1fr);
	gap: 1px;
	margin: 10px 0 12px;
	background: var(--rule);
	border: 1px solid var(--rule);
	border-radius: 4px;
	overflow: hidden;
}

.meta > div {
	padding: 5px 10px;
	background: var(--band);
}

.meta dt {
	font-size: 8.5px;
	font-weight: 600;
	letter-spacing: 0.08em;
	text-transform: uppercase;
	color: var(--muted);
}

.meta dd {
	margin: 1px 0 0;
	font-size: 12px;
	font-weight: 600;
}

.loads.split {
	display: grid;
	grid-template-columns: 1fr 1fr;
	gap: 0 18px;
}

.loads table {
	width: 100%;
	border-collapse: collapse;
	align-self: start;
}

.loads th {
	padding: 0 6px 3px;
	font-size: 8.5px;
	font-weight: 600;
	letter-spacing: 0.08em;
	text-align: left;
	text-transform: uppercase;
	color: var(--muted);
	border-bottom: 1.5px solid var(--ink);
}

.loads td {
	padding: 2.5px 6px;
	border-bottom: 1px solid var(--rule);
	white-space: nowrap;
}

.loads tbody tr:nth-child(even) td {
	background: var(--band);
}

.loads .num {
	text-align: right;
	font-variant-numeric: tabular-nums;
}

.loads .date {
	color: var(--muted);
}

.loads .store {
	max-width: 1.3in;
	overflow: hidden;
	text-overflow: ellipsis;
	font-weight: 500;
}

.loads .muted {
	color: #b3bac4;
}

.empty {
	margin: 8px 0;
	color: var(--muted);
	text-align: center;
}

.totals {
	display: flex;
	align-items: stretch;
	gap: 14px;
	margin-top: 12px;
	padding-top: 10px;
	border-top: 1.5px solid var(--ink);
}

.breakdown {
	flex: 1;
	display: grid;
	grid-template-columns: repeat(3, 1fr);
	gap: 10px;
}

.breakdown > div,
.net {
	display: flex;
	flex-direction: column;
	justify-content: center;
}

.breakdown span,
.net span {
	font-size: 8.5px;
	font-weight: 600;
	letter-spacing: 0.08em;
	text-transform: uppercase;
	color: var(--muted);
}

.breakdown strong {
	font-size: 13px;
	font-weight: 600;
	font-variant-numeric: tabular-nums;
}

.net {
	min-width: 1.7in;
	padding: 6px 14px;
	border-radius: 4px;
	background: var(--accent);
	text-align: right;
}

.net span {
	color: rgba(255, 255, 255, 0.8);
}

.net strong {
	font-size: 20px;
	font-weight: 700;
	color: #fff;
	font-variant-numeric: tabular-nums;
}
</style>

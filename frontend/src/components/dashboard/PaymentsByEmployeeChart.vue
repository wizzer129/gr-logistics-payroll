<script setup lang="ts">
import { computed } from 'vue';
import { Bar } from 'vue-chartjs';
import {
	BarElement,
	CategoryScale,
	Chart as ChartJS,
	Legend,
	LinearScale,
	Title,
	Tooltip,
	type ChartData,
	type LegendItem,
	type TooltipItem,
} from 'chart.js';
import { assignStableColors, useChartColors } from '../../utils/chart-colors';
import { formatCurrency } from '../../utils/currency';

ChartJS.register(Title, Tooltip, Legend, CategoryScale, LinearScale, BarElement);

const props = defineProps<{
	data: { employeeId: string; firstName: string; lastName: string; total: number }[];
}>();

const colors = useChartColors();

// Color follows each employee's identity, not their rank in this total-sorted list — so
// a given employee keeps the same color even as the "by total" order shifts week to week.
const employeeColors = computed(() =>
	assignStableColors(
		props.data.map((row) => row.employeeId),
		colors.categorical.value,
	),
);

const chartData = computed(() => ({
	labels: props.data.map((row) => `${row.firstName} ${row.lastName}`),
	datasets: [
		{
			data: props.data.map((row) => row.total),
			backgroundColor: props.data.map(
				(row) => employeeColors.value.get(row.employeeId) ?? colors.accent.value,
			),
			borderRadius: 4,
		},
	],
}));

const chartOptions = computed(() => ({
	indexAxis: 'y' as const,
	responsive: true,
	maintainAspectRatio: false,
	plugins: {
		legend: {
			display: true,
			position: 'bottom' as const,
			labels: {
				color: colors.textPrimary.value,
				// One legend entry per employee (per bar), not per dataset — Chart.js's
				// default legend is dataset-level, so build it from the data points instead.
				generateLabels: (chart: ChartJS): LegendItem[] => {
					const data = chart.data as ChartData<'bar'>;
					const colorsArr = data.datasets[0]?.backgroundColor as string[];
					return data.labels!.map((label, i) => ({
						text: label as string,
						// The legend plugin's draw step reads fontColor off each LegendItem
						// (not options.labels.color) — our custom generateLabels must set it,
						// or Chart.js falls back to its own default instead of the theme color.
						fontColor: colors.textPrimary.value,
						fillStyle: colorsArr[i],
						strokeStyle: colorsArr[i],
						hidden:
							(chart.getDatasetMeta(0).data[i] as unknown as { hidden?: boolean })
								?.hidden ?? false,
						index: i,
					}));
				},
			},
			onClick: (_event: unknown, item: LegendItem, legend: { chart: ChartJS }) => {
				const meta = legend.chart.getDatasetMeta(0);
				const element = meta.data[item.index!] as unknown as { hidden?: boolean };
				element.hidden = !element.hidden;
				legend.chart.update();
			},
		},
		tooltip: {
			callbacks: {
				label: (ctx: TooltipItem<'bar'>) => formatCurrency(ctx.raw as number),
			},
		},
	},
	scales: {
		x: {
			ticks: {
				color: () => colors.textMuted.value,
				callback: (v: number | string) => formatCurrency(Number(v)),
			},
			grid: { color: () => colors.grid.value },
		},
		y: {
			ticks: { color: () => colors.textPrimary.value },
			grid: { display: false },
		},
	},
}));
</script>

<template>
	<div style="height: 320px">
		<!-- Legend text color is static (Chart.js's legend plugin has no scriptable
		color option, unlike scale ticks) so it's baked in at chart creation. Keying on
		the theme forces a full remount on toggle instead of a stale in-place update. -->
		<Bar
			:key="colors.isDark.value ? 'dark' : 'light'"
			:data="chartData"
			:options="chartOptions"
		/>
	</div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { Line } from 'vue-chartjs';
import { useI18n } from 'vue-i18n';
import {
	CategoryScale,
	Chart as ChartJS,
	Legend,
	LinearScale,
	LineElement,
	PointElement,
	Title,
	Tooltip,
	type TooltipItem,
} from 'chart.js';
import { assignStableColors, useChartColors } from '../../utils/chart-colors';
import { formatCurrency } from '../../utils/currency';

ChartJS.register(Title, Tooltip, Legend, CategoryScale, LinearScale, LineElement, PointElement);

const props = defineProps<{
	data: {
		period: string;
		employeeId: string;
		firstName: string;
		lastName: string;
		total: number;
	}[];
}>();

const { locale } = useI18n();
const colors = useChartColors();

function formatPeriod(period: string): string {
	const [year, month] = period.split('-').map(Number);
	return new Intl.DateTimeFormat(locale.value, { month: 'short' }).format(
		new Date(year, month - 1, 1),
	);
}

const periods = computed(() => [...new Set(props.data.map((row) => row.period))].sort());

// Same id set and palette as PaymentsByEmployeeChart, so each employee's line matches
// their bar color.
const employeeColors = computed(() =>
	assignStableColors(
		props.data.map((row) => row.employeeId),
		colors.categorical.value,
	),
);

const chartData = computed(() => {
	const employees = new Map<string, { name: string; totals: Map<string, number> }>();
	for (const row of props.data) {
		let employee = employees.get(row.employeeId);
		if (!employee) {
			employee = { name: `${row.firstName} ${row.lastName}`, totals: new Map() };
			employees.set(row.employeeId, employee);
		}
		employee.totals.set(row.period, row.total);
	}

	return {
		labels: periods.value.map(formatPeriod),
		datasets: [...employees].map(([employeeId, employee]) => {
			const color = employeeColors.value.get(employeeId) ?? colors.accent.value;
			return {
				label: employee.name,
				// A month with no completed paycheck is a real $0 for that employee.
				data: periods.value.map((period) => employee.totals.get(period) ?? 0),
				borderColor: color,
				backgroundColor: color,
				tension: 0.3,
				pointRadius: 3,
				borderWidth: 2,
			};
		}),
	};
});

const chartOptions = computed(() => ({
	responsive: true,
	maintainAspectRatio: false,
	interaction: { mode: 'index' as const, intersect: false },
	plugins: {
		legend: {
			display: true,
			position: 'bottom' as const,
			labels: { color: colors.textPrimary.value, usePointStyle: true },
		},
		tooltip: {
			callbacks: {
				label: (ctx: TooltipItem<'line'>) =>
					`${ctx.dataset.label}: ${formatCurrency(ctx.raw as number)}`,
			},
		},
	},
	scales: {
		x: {
			ticks: { color: () => colors.textMuted.value },
			grid: { display: false },
		},
		y: {
			ticks: {
				color: () => colors.textMuted.value,
				callback: (v: number | string) => formatCurrency(Number(v)),
			},
			grid: { color: () => colors.grid.value },
		},
	},
}));
</script>

<template>
	<div style="height: 320px">
		<!-- Legend text color isn't scriptable, so rebuild the chart on theme change. -->
		<Line
			:key="colors.isDark.value ? 'dark' : 'light'"
			:data="chartData"
			:options="chartOptions"
		/>
	</div>
</template>

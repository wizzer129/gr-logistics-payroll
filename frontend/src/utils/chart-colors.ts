import { computed } from 'vue';
import { vuetify } from '../plugins/vuetify';

// Fixed categorical hue order (never cycled/generated) — validated for CVD-safe adjacent
// pairs, light and dark variants of the same 8 hues. See the data-viz palette reference.
const CATEGORICAL_LIGHT = [
	'#2a78d6', // blue
	'#eb6834', // orange
	'#1baf7a', // aqua
	'#eda100', // yellow
	'#e87ba4', // magenta
	'#008300', // green
	'#4a3aa7', // violet
	'#e34948', // red
];
const CATEGORICAL_DARK = [
	'#3987e5',
	'#d95926',
	'#199e70',
	'#c98500',
	'#d55181',
	'#008300',
	'#9085e9',
	'#e66767',
];

/** Theme-aware chart tokens — single sequential blue hue, matching the project's
 * data-viz palette (light `#2a78d6` / dark `#3987e5`), reactive to the live theme toggle. */
export function useChartColors() {
	const isDark = computed(() => vuetify.theme.global.name.value === 'dark');

	return {
		isDark,
		accent: computed(() => (isDark.value ? '#3987e5' : '#2a78d6')),
		textPrimary: computed(() => (isDark.value ? '#ffffff' : '#0b0b0b')),
		textMuted: computed(() => '#898781'),
		grid: computed(() => (isDark.value ? '#2c2c2a' : '#e1e0d9')),
		categorical: computed(() => (isDark.value ? CATEGORICAL_DARK : CATEGORICAL_LIGHT)),
	};
}

/** Assigns each id a color from the fixed categorical order, stable across re-renders
 * regardless of the order `ids` arrives in (e.g. a list re-sorted by rank/total) — the
 * color follows the entity's identity, never its position in a sorted list. Past 8
 * distinct ids the palette repeats rather than generating a 9th hue (see the data-viz
 * skill: never solve "too many series" by generating more hues). */
export function assignStableColors(ids: string[], palette: string[]): Map<string, string> {
	const stableOrder = [...new Set(ids)].sort();
	return new Map(stableOrder.map((id, i) => [id, palette[i % palette.length]]));
}

import type { ThemeDefinition } from 'vuetify';

/**
 * The color themes a user can pick in Settings (saved per user, `users.theme`). Each family
 * has a light and a dark variant, so the app bar's dark-mode toggle works with any of them.
 * Mirrors the backend's `theme::Theme` and `users.theme`'s CHECK (migrations 0015, 0016) —
 * add a family in all three.
 */
export type ThemeFamily = 'default' | 'catppuccin' | 'dracula' | 'autumn' | 'stormy';
export type ThemeMode = 'light' | 'dark';

export const THEME_FAMILIES: ThemeFamily[] = [
	'default',
	'catppuccin',
	'dracula',
	'autumn',
	'stormy',
];

/** The Vuetify theme name for a family in a mode. `default` is Vuetify's own `light`/`dark`. */
export function themeName(family: ThemeFamily, mode: ThemeMode): string {
	return family === 'default' ? mode : `${family}-${mode}`;
}

export function isThemeFamily(value: unknown): value is ThemeFamily {
	return THEME_FAMILIES.includes(value as ThemeFamily);
}

// Catppuccin: Frappé is its dark flavor; Latte, its light one, is the light variant.
// https://catppuccin.com/palette
const catppuccinLight: ThemeDefinition = {
	dark: false,
	colors: {
		background: '#eff1f5', // base
		surface: '#e6e9ef', // mantle
		'surface-bright': '#eff1f5',
		'surface-variant': '#4c4f69',
		'on-surface-variant': '#eff1f5',
		'on-background': '#4c4f69', // text
		'on-surface': '#4c4f69',
		primary: '#1e66f5', // blue
		secondary: '#8839ef', // mauve
		error: '#d20f39', // red
		info: '#209fb5', // sapphire
		success: '#40a02b', // green
		warning: '#df8e1d', // yellow
	},
};

const catppuccinDark: ThemeDefinition = {
	dark: true,
	colors: {
		background: '#232634', // crust
		surface: '#303446', // base
		'surface-bright': '#414559', // surface0
		'surface-variant': '#c6d0f5',
		'on-surface-variant': '#303446',
		'on-background': '#c6d0f5', // text
		'on-surface': '#c6d0f5',
		primary: '#8caaee', // blue
		secondary: '#ca9ee6', // mauve
		error: '#e78284', // red
		info: '#85c1dc', // sapphire
		success: '#a6d189', // green
		warning: '#e5c890', // yellow
	},
};

// Dracula, and Alucard — Dracula's official light variant. https://draculatheme.com/spec
const draculaLight: ThemeDefinition = {
	dark: false,
	colors: {
		background: '#fffbeb',
		surface: '#fffdf5',
		'surface-bright': '#fffbeb',
		'surface-variant': '#1f1f1f',
		'on-surface-variant': '#fffbeb',
		'on-background': '#1f1f1f',
		'on-surface': '#1f1f1f',
		primary: '#644ac9', // purple
		secondary: '#a3144d', // pink
		error: '#cb3a2a', // red
		info: '#036a96', // cyan
		success: '#14710a', // green
		warning: '#a34d14', // orange
	},
};

const draculaDark: ThemeDefinition = {
	dark: true,
	colors: {
		background: '#21222c',
		surface: '#282a36', // background
		'surface-bright': '#44475a', // current line
		'surface-variant': '#f8f8f2',
		'on-surface-variant': '#282a36',
		'on-background': '#f8f8f2', // foreground
		'on-surface': '#f8f8f2',
		primary: '#bd93f9', // purple
		secondary: '#ff79c6', // pink
		error: '#ff5555', // red
		info: '#8be9fd', // cyan
		success: '#50fa7b', // green
		warning: '#ffb86c', // orange
	},
};

// Autumn: our own — burnt orange, brick and olive on warm paper / dark walnut.
const autumnLight: ThemeDefinition = {
	dark: false,
	colors: {
		background: '#f7f0e6',
		surface: '#fffaf3',
		'surface-bright': '#fffdf9',
		'surface-variant': '#3b2a1e',
		'on-surface-variant': '#f7f0e6',
		'on-background': '#3b2a1e',
		'on-surface': '#3b2a1e',
		primary: '#b5541b', // burnt orange
		secondary: '#7a4a2a', // chestnut
		error: '#a8322d', // brick
		info: '#3f6e7a', // slate teal
		success: '#5f7f2a', // olive
		warning: '#b7791f', // mustard
	},
};

const autumnDark: ThemeDefinition = {
	dark: true,
	colors: {
		background: '#1b1511',
		surface: '#271f19',
		'surface-bright': '#3a2e25',
		'surface-variant': '#f1e6d8',
		'on-surface-variant': '#271f19',
		'on-background': '#f1e6d8',
		'on-surface': '#f1e6d8',
		primary: '#e0863f', // pumpkin
		secondary: '#c49a6c', // oak
		error: '#e06a5a', // brick
		info: '#7fb0bc', // slate teal
		success: '#9bb562', // olive
		warning: '#e4b04a', // mustard
	},
};

// Stormy Coral: storm grays with a coral accent. The palette's five colors are #f4f6f8 (mist),
// #b7bec7 (cloud), #59626b (slate), #f05b5b (coral) and #1b1f24 (night); the status colors
// (info, success, warning, error) are muted additions in the same mood, since it has none.
const stormyLight: ThemeDefinition = {
	dark: false,
	colors: {
		background: '#f4f6f8', // mist
		surface: '#ffffff',
		'surface-bright': '#ffffff',
		'surface-variant': '#1b1f24', // night
		'on-surface-variant': '#f4f6f8',
		'on-background': '#1b1f24',
		'on-surface': '#1b1f24',
		primary: '#f05b5b', // coral
		secondary: '#59626b', // slate
		error: '#c62f3b',
		info: '#3f6f94',
		success: '#2f7d55',
		warning: '#b7791f',
	},
};

const stormyDark: ThemeDefinition = {
	dark: true,
	colors: {
		background: '#1b1f24', // night
		surface: '#242a31',
		'surface-bright': '#2f363e',
		'surface-variant': '#b7bec7', // cloud
		'on-surface-variant': '#1b1f24',
		'on-background': '#f4f6f8', // mist
		'on-surface': '#f4f6f8',
		primary: '#f05b5b', // coral
		secondary: '#b7bec7', // cloud
		error: '#ff8a8a',
		info: '#8fb6d4',
		success: '#6cc392',
		warning: '#e8b44e',
	},
};

/** Every theme but `default`'s, which are Vuetify's built-in `light` and `dark`. */
export const CUSTOM_THEMES: Record<string, ThemeDefinition> = {
	[themeName('catppuccin', 'light')]: catppuccinLight,
	[themeName('catppuccin', 'dark')]: catppuccinDark,
	[themeName('dracula', 'light')]: draculaLight,
	[themeName('dracula', 'dark')]: draculaDark,
	[themeName('autumn', 'light')]: autumnLight,
	[themeName('autumn', 'dark')]: autumnDark,
	[themeName('stormy', 'light')]: stormyLight,
	[themeName('stormy', 'dark')]: stormyDark,
};

import 'vuetify/styles';
import '@mdi/font/css/materialdesignicons.css';
import { ref } from 'vue';
import { createVuetify } from 'vuetify';
import { en, es } from 'vuetify/locale';
import {
	CUSTOM_THEMES,
	isThemeFamily,
	themeName,
	type ThemeFamily,
	type ThemeMode,
} from './themes';

// Light/dark is per device (the app bar toggle). The family is per user (Settings, saved on
// the server); it's cached here too so the login page and a refresh start in the right one.
const MODE_STORAGE_KEY = 'gr-logistics-theme';
const FAMILY_STORAGE_KEY = 'gr-logistics-theme-family';

function detectMode(): ThemeMode {
	const stored = localStorage.getItem(MODE_STORAGE_KEY);
	if (stored === 'light' || stored === 'dark') return stored;

	return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

function cachedFamily(): ThemeFamily {
	const stored = localStorage.getItem(FAMILY_STORAGE_KEY);
	return isThemeFamily(stored) ? stored : 'default';
}

const mode = ref<ThemeMode>(detectMode());
const family = ref<ThemeFamily>(cachedFamily());

export const vuetify = createVuetify({
	locale: {
		locale: 'en',
		fallback: 'en',
		messages: { en, es },
	},
	theme: {
		defaultTheme: themeName(family.value, mode.value),
		themes: CUSTOM_THEMES,
	},
	defaults: {
		VTextField: { variant: 'outlined' },
		VSelect: { variant: 'solo-filled' },
		VAutocomplete: { variant: 'solo-filled' },
		VCombobox: { variant: 'solo-filled' },
	},
});

function apply(): void {
	vuetify.theme.global.name.value = themeName(family.value, mode.value);
}

export function currentMode(): ThemeMode {
	return mode.value;
}

export function setMode(value: ThemeMode): void {
	mode.value = value;
	localStorage.setItem(MODE_STORAGE_KEY, value);
	apply();
}

export function toggleTheme(): void {
	setMode(mode.value === 'dark' ? 'light' : 'dark');
}

export function currentFamily(): ThemeFamily {
	return family.value;
}

/** Applies a theme family locally. Saving it for the user is `useAuthStore().saveTheme()`. */
export function setFamily(value: ThemeFamily): void {
	family.value = value;
	localStorage.setItem(FAMILY_STORAGE_KEY, value);
	apply();
}

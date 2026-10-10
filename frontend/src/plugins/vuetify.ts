import 'vuetify/styles';
import '@mdi/font/css/materialdesignicons.css';
import { createVuetify } from 'vuetify';
import { en, es } from 'vuetify/locale';

const THEME_STORAGE_KEY = 'gr-logistics-theme';

export type ThemeName = 'light' | 'dark';

function detectTheme(): ThemeName {
	const stored = localStorage.getItem(THEME_STORAGE_KEY);
	if (stored === 'light' || stored === 'dark') return stored;

	return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

export const vuetify = createVuetify({
	locale: {
		locale: 'en',
		fallback: 'en',
		messages: { en, es },
	},
	theme: {
		defaultTheme: detectTheme(),
	},
});

export function currentTheme(): ThemeName {
	return vuetify.theme.global.name.value as ThemeName;
}

export function setTheme(theme: ThemeName): void {
	vuetify.theme.global.name.value = theme;
	localStorage.setItem(THEME_STORAGE_KEY, theme);
}

export function toggleTheme(): void {
	setTheme(currentTheme() === 'dark' ? 'light' : 'dark');
}

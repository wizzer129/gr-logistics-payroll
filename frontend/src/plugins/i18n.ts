import type { WritableComputedRef } from 'vue';
import { createI18n } from 'vue-i18n';
import en from '../locales/en.json';
import es from '../locales/es.json';

const STORAGE_KEY = 'gr-logistics-locale';

function detectLocale(): 'en' | 'es' {
	const stored = localStorage.getItem(STORAGE_KEY);
	if (stored === 'en' || stored === 'es') return stored;

	return navigator.language.startsWith('es') ? 'es' : 'en';
}

const currencyFormat = { currency: { style: 'currency', currency: 'USD' } } as const;

const messages: Record<string, typeof en> = { en, es };
const numberFormats: Record<string, typeof currencyFormat> = {
	en: currencyFormat,
	es: currencyFormat,
};

export const i18n = createI18n({
	legacy: false,
	locale: detectLocale(),
	fallbackLocale: 'en',
	messages,
	numberFormats,
});

export function localeRef(): WritableComputedRef<string> {
	return i18n.global.locale as unknown as WritableComputedRef<string>;
}

export function setLocale(locale: 'en' | 'es'): void {
	localeRef().value = locale;
	localStorage.setItem(STORAGE_KEY, locale);
}

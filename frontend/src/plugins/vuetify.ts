import 'vuetify/styles';
import '@mdi/font/css/materialdesignicons.css';
import { createVuetify } from 'vuetify';
import { en, es } from 'vuetify/locale';

export const vuetify = createVuetify({
	locale: {
		locale: 'en',
		fallback: 'en',
		messages: { en, es },
	},
	theme: {
		defaultTheme: 'light',
	},
});

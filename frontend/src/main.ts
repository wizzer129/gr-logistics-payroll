import { createApp, watch } from 'vue';
import { createPinia } from 'pinia';
import App from './App.vue';
import router from './router';
import { vuetify } from './plugins/vuetify';
import { i18n, localeRef } from './plugins/i18n';

const app = createApp(App);

app.use(createPinia());
app.use(router);
app.use(vuetify);
app.use(i18n);

watch(
	localeRef(),
	(locale) => {
		vuetify.locale.current.value = locale;
	},
	{ immediate: true },
);

app.mount('#app');

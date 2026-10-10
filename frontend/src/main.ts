import { createApp, watch } from 'vue';
import { createPinia } from 'pinia';
import App from './App.vue';
import router from './router';
import { vuetify } from './plugins/vuetify';
import { i18n, localeRef } from './plugins/i18n';
import { useAuthStore } from './store/auth-store';

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

// Reconcile the locally-cached user with the session cookie before the router's
// first navigation, so a page refresh doesn't briefly/incorrectly bounce to login.
useAuthStore()
	.restoreSession()
	.finally(() => app.mount('#app'));

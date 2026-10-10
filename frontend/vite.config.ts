import { fileURLToPath, URL } from 'node:url';
import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import vuetify from 'vite-plugin-vuetify';
import { VitePWA } from 'vite-plugin-pwa';

export default defineConfig({
	plugins: [
		vue(),
		vuetify({ autoImport: true }),
		VitePWA({
			registerType: 'autoUpdate',
			manifest: {
				name: 'GR Logistics',
				short_name: 'GR Logistics',
				description: 'Employee payment tracking portal',
				theme_color: '#1976d2',
				icons: [],
			},
		}),
	],
	resolve: {
		alias: {
			'@': fileURLToPath(new URL('./src', import.meta.url)),
		},
	},
	server: {
		proxy: {
			'/api': 'http://localhost:3000',
		},
	},
	test: {
		environment: 'happy-dom',
		include: ['unit-tests/**/*.test.ts'],
	},
});

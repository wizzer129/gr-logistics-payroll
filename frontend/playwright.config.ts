import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
	testDir: 'e2e-tests',
	testMatch: '**/*.spec.ts',
	fullyParallel: true,
	reporter: 'list',
	use: {
		baseURL: 'http://localhost:5173',
		trace: 'on-first-retry',
	},
	projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
	webServer: {
		command: 'bun run dev',
		url: 'http://localhost:5173',
		reuseExistingServer: !process.env.CI,
		timeout: 30_000,
	},
});

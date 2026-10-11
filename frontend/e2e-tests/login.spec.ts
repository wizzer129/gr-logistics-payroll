import { expect, test } from '@playwright/test';

test('shows the login form for an unauthenticated visitor', async ({ page }) => {
	await page.goto('/');
	await expect(page).toHaveURL(/\/login(\?.*)?$/);
	await expect(page.locator('input[type="email"]')).toBeVisible();
	await expect(page.locator('input[type="password"]')).toBeVisible();
});

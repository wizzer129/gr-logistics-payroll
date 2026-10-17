import { expect, test } from '@playwright/test';

// The pages only; the backend flow is covered by backend/tests/it/password_reset.rs.

test('the login page links to the forgot-password form', async ({ page }) => {
	await page.goto('/login');
	await page.getByRole('link', { name: 'Forgot password?' }).click();
	await expect(page).toHaveURL(/\/forgot-password$/);
	await expect(page.locator('input[type="email"]')).toBeVisible();
	await expect(page.getByRole('button', { name: 'Send reset link' })).toBeDisabled();
});

test('a reset page without a token says the link is invalid', async ({ page }) => {
	await page.goto('/reset-password');
	await expect(page.getByText('This reset link is invalid')).toBeVisible();
	await expect(page.getByRole('link', { name: 'Request a new link' })).toBeVisible();
});

test('the token is taken out of the address bar', async ({ page }) => {
	await page.goto('/reset-password?token=abc123');
	await expect(page).toHaveURL(/\/reset-password$/);
	await expect(page.getByLabel('New password', { exact: true })).toBeVisible();
});

test('the new password must be long enough and match', async ({ page }) => {
	await page.goto('/reset-password?token=abc123');
	await page.getByLabel('New password', { exact: true }).fill('short');
	await page.getByLabel('Confirm new password').fill('different');
	await page.getByRole('button', { name: 'Save new password' }).click();
	await expect(page.getByText('Use 12 to 128 characters.').first()).toBeVisible();
	await expect(page.getByText("The passwords don't match.")).toBeVisible();
});

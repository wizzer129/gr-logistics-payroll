import { test } from '@playwright/test';

test('sample legend text pixel color in dark mode', async ({ page }) => {
	await page.goto('/login');
	await page.fill('input[type="email"]', 'smoketest@gr-logistics.local');
	await page.fill('input[type="password"]', 'smoketest123');
	await page.click('button[type="submit"]');
	await page.waitForURL('**/dashboard', { timeout: 10000 }).catch(() => {});
	await page.waitForTimeout(1000);

	await page.evaluate(() => localStorage.setItem('gr-logistics-theme', 'dark'));
	await page.reload();
	await page.waitForTimeout(1500);

	const result = await page.evaluate(() => {
		const canvases = Array.from(document.querySelectorAll('canvas'));
		const barCanvas = canvases[0] as HTMLCanvasElement;
		const ctx = barCanvas.getContext('2d')!;
		const img = ctx.getImageData(0, 0, barCanvas.width, barCanvas.height).data;

		// Scan the bottom region (legend) for the darkest and lightest non-background pixels
		const h = barCanvas.height;
		const w = barCanvas.width;
		let darkest = { r: 255, g: 255, b: 255, sum: 765 };
		let lightest = { r: 0, g: 0, b: 0, sum: 0 };
		for (let y = Math.floor(h * 0.75); y < h; y++) {
			for (let x = 0; x < w; x++) {
				const i = (y * w + x) * 4;
				const r = img[i],
					g = img[i + 1],
					b = img[i + 2],
					a = img[i + 3];
				if (a === 0) continue;
				const sum = r + g + b;
				// skip near-background colors (dark surface ~ #1e1e1e-ish) and skip colored swatches
				const isGray = Math.abs(r - g) < 10 && Math.abs(g - b) < 10;
				if (!isGray) continue;
				if (sum < 80) continue; // too close to background, not text
				if (sum < darkest.sum) darkest = { r, g, b, sum };
				if (sum > lightest.sum) lightest = { r, g, b, sum };
			}
		}
		return {
			canvasSize: { w, h },
			darkestGrayText: darkest,
			lightestGrayText: lightest,
			devicePixelRatio: window.devicePixelRatio,
		};
	});

	console.log(JSON.stringify(result, null, 2));
});

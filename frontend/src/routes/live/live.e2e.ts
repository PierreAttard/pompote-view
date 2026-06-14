import { expect, test } from '@playwright/test';

/**
 * Smoke: changing the timeframe selector refetches the candles and refreshes the
 * chart (issue #18 acceptance criterion).
 *
 * The live page loads its selectors (strategies/timeframes) server-side from the
 * viz backend, so this smoke needs the backend running (CI scopes e2e to a later
 * lot — see `.github/workflows/check.yml`). When the backend is unreachable the
 * selectors never render; the test then skips rather than failing. The candles
 * call is stubbed at the browser boundary so the chart is deterministic.
 */
const CANDLES = [
	{ ts: '2026-06-01T00:00:00Z', o: 100, h: 110, l: 95, c: 105, v: 12 },
	{ ts: '2026-06-01T01:00:00Z', o: 105, h: 120, l: 104, c: 118, v: 30 },
	{ ts: '2026-06-01T02:00:00Z', o: 118, h: 119, l: 100, c: 102, v: 21 }
];

test('changing the timeframe refreshes the chart', async ({ page }) => {
	await page.route('**/api/candles**', (route) =>
		route.fulfill({ json: CANDLES, headers: { 'content-type': 'application/json' } })
	);

	await page.goto('/live?strategy=smoke&exchange=binance&symbol=BTCUSDT&timeframe=1h&preset=24h');

	const selectors = page.getByTestId('live-selectors');
	if (!(await selectors.isVisible().catch(() => false))) {
		test.skip(true, 'viz backend not reachable — live selectors did not load');
	}

	// The chart renders from the stubbed candles.
	await expect(page.getByTestId('live-chart')).toBeVisible();
	await expect(page.getByTestId('chart').locator('canvas').first()).toBeVisible();

	// Pick a timeframe different from the current one.
	const timeframe = page.getByTestId('select-timeframe');
	const options = await timeframe
		.locator('option')
		.evaluateAll((els) => els.map((el) => (el as HTMLOptionElement).value));
	const next = options.find((v) => v !== '1h');
	test.skip(!next, 'backend exposes a single timeframe — nothing to switch to');

	// Changing the timeframe must trigger a fresh candles request for that value.
	const candlesRequest = page.waitForRequest(
		(req) => req.url().includes('/api/candles') && req.url().includes(`timeframe=${next}`)
	);
	await timeframe.selectOption(next!);
	await candlesRequest;

	await expect(page.getByTestId('chart').locator('canvas').first()).toBeVisible();
});

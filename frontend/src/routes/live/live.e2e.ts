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

test('an over-cap (timeframe, range) shows the depth warning and fires no candles request', async ({
	page
}) => {
	// Fail loudly if a candles request leaves: the over-cap selection must be
	// caught on the UI before any backend call (issue #19).
	let candlesCalled = false;
	await page.route('**/api/candles**', (route) => {
		candlesCalled = true;
		return route.fulfill({ json: CANDLES, headers: { 'content-type': 'application/json' } });
	});

	// 7d at 1m = 10080 buckets, well over the 5000 cap.
	await page.goto('/live?strategy=smoke&exchange=binance&symbol=BTCUSDT&timeframe=1m&preset=7d');

	const selectors = page.getByTestId('live-selectors');
	if (!(await selectors.isVisible().catch(() => false))) {
		test.skip(true, 'viz backend not reachable — live selectors did not load');
	}

	await expect(page.getByTestId('live-depth-warning')).toBeVisible();
	await expect(page.getByTestId('live-chart')).toHaveCount(0);
	// Give any stray request a beat to fire before asserting none did.
	await page.waitForTimeout(300);
	expect(candlesCalled).toBe(false);
});

test('the live chart fetches decision annotations (orders + decisions)', async ({ page }) => {
	await page.route('**/api/candles**', (route) =>
		route.fulfill({ json: CANDLES, headers: { 'content-type': 'application/json' } })
	);
	await page.route('**/api/strategies/*/orders**', (route) =>
		route.fulfill({ json: [], headers: { 'content-type': 'application/json' } })
	);
	await page.route('**/api/strategies/*/decisions**', (route) =>
		route.fulfill({ json: [], headers: { 'content-type': 'application/json' } })
	);

	const ordersRequest = page.waitForRequest((r) => /\/api\/strategies\/.+\/orders/.test(r.url()));
	const decisionsRequest = page.waitForRequest((r) =>
		/\/api\/strategies\/.+\/decisions/.test(r.url())
	);

	await page.goto('/live?strategy=smoke&exchange=binance&symbol=BTCUSDT&timeframe=1h&preset=24h');

	const selectors = page.getByTestId('live-selectors');
	if (!(await selectors.isVisible().catch(() => false))) {
		test.skip(true, 'viz backend not reachable — live selectors did not load');
	}

	await expect(page.getByTestId('live-chart')).toBeVisible();
	// The annotation queries fire so markers + decision tooltips have their data.
	await ordersRequest;
	await decisionsRequest;
});

test('clicking a decision bar opens the detail panel; Escape closes it', async ({ page }) => {
	// One decision per candle bucket so any click in the data area resolves.
	const ORDERS = CANDLES.map((c, i) => ({
		order_id: `o${i}`,
		decision_id: `d${i}`,
		side: i % 2 === 0 ? 'buy' : 'sell',
		quantity: 1,
		price: c.c,
		status: 'filled',
		created_at: c.ts
	}));
	const DECISIONS = CANDLES.map((c, i) => ({
		decision_id: `d${i}`,
		session_id: 's1',
		created_at: c.ts,
		reason: `decision ${i}`,
		orders_count: 1,
		market_context: { rsi: 30 + i }
	}));

	await page.route('**/api/candles**', (route) =>
		route.fulfill({ json: CANDLES, headers: { 'content-type': 'application/json' } })
	);
	await page.route('**/api/strategies/*/orders**', (route) =>
		route.fulfill({ json: ORDERS, headers: { 'content-type': 'application/json' } })
	);
	await page.route('**/api/strategies/*/decisions**', (route) =>
		route.fulfill({ json: DECISIONS, headers: { 'content-type': 'application/json' } })
	);
	await page.route('**/api/strategies/*/fills**', (route) =>
		route.fulfill({ json: [], headers: { 'content-type': 'application/json' } })
	);

	await page.goto('/live?strategy=smoke&exchange=binance&symbol=BTCUSDT&timeframe=1h&preset=24h');

	const selectors = page.getByTestId('live-selectors');
	if (!(await selectors.isVisible().catch(() => false))) {
		test.skip(true, 'viz backend not reachable — live selectors did not load');
	}

	await expect(page.getByTestId('chart').locator('canvas').first()).toBeVisible();

	// Click the chart; the click resolves to a bar, which carries a decision.
	await page.getByTestId('chart').click();
	await expect(page.getByTestId('decision-panel')).toBeVisible();
	await expect(page.getByTestId('panel-reason')).toContainText('decision');

	// Escape closes it (issue #22 acceptance criterion).
	await page.keyboard.press('Escape');
	await expect(page.getByTestId('decision-panel')).toHaveCount(0);
});

test('indicator toggles enable a sub-chart from decision snapshots', async ({ page }) => {
	const DECISIONS = [
		{
			decision_id: 'd0',
			session_id: 's1',
			created_at: CANDLES[0].ts,
			reason: 'd0',
			orders_count: 0,
			market_context: { rsi: 28, adx: 30 }
		}
	];

	await page.route('**/api/candles**', (route) =>
		route.fulfill({ json: CANDLES, headers: { 'content-type': 'application/json' } })
	);
	await page.route('**/api/strategies/*/orders**', (route) =>
		route.fulfill({ json: [], headers: { 'content-type': 'application/json' } })
	);
	await page.route('**/api/strategies/*/decisions**', (route) =>
		route.fulfill({ json: DECISIONS, headers: { 'content-type': 'application/json' } })
	);

	await page.goto('/live?strategy=smoke&exchange=binance&symbol=BTCUSDT&timeframe=1h&preset=24h');

	const selectors = page.getByTestId('live-selectors');
	if (!(await selectors.isVisible().catch(() => false))) {
		test.skip(true, 'viz backend not reachable — live selectors did not load');
	}

	await expect(page.getByTestId('chart').locator('canvas').first()).toBeVisible();

	// Toggles are built generically from the snapshot's numeric fields (#24).
	await expect(page.getByTestId('live-indicator-toggles')).toBeVisible();
	const rsi = page.getByRole('button', { name: /rsi/i });
	await expect(rsi).toHaveAttribute('aria-pressed', 'false');
	await rsi.click();
	await expect(rsi).toHaveAttribute('aria-pressed', 'true');
	// The chart keeps rendering with the extra pane.
	await expect(page.getByTestId('chart').locator('canvas').first()).toBeVisible();
});

test('the volume profile renders beside the chart with a POC', async ({ page }) => {
	await page.route('**/api/candles**', (route) =>
		route.fulfill({ json: CANDLES, headers: { 'content-type': 'application/json' } })
	);

	await page.goto('/live?strategy=smoke&exchange=binance&symbol=BTCUSDT&timeframe=1h&preset=24h');

	const selectors = page.getByTestId('live-selectors');
	if (!(await selectors.isVisible().catch(() => false))) {
		test.skip(true, 'viz backend not reachable — live selectors did not load');
	}

	await expect(page.getByTestId('chart').locator('canvas').first()).toBeVisible();
	// Volume profile (#25) is aggregated client-side from the candles.
	await expect(page.getByTestId('volume-profile')).toBeVisible();
	await expect(page.getByTestId('vp-poc')).toBeVisible();
});

test('polls every 10s and grows the chart with a new candle (#26)', async ({ page }) => {
	const EXTRA = { ts: '2026-06-01T03:00:00Z', o: 102, h: 130, l: 101, c: 125, v: 9 };
	let calls = 0;
	const fromParams: (string | null)[] = [];
	await page.route('**/api/candles**', (route) => {
		fromParams.push(new URL(route.request().url()).searchParams.get('from'));
		calls += 1;
		// The poll returns the original window plus one fresh candle.
		const data = calls === 1 ? CANDLES : [...CANDLES, EXTRA];
		return route.fulfill({ json: data, headers: { 'content-type': 'application/json' } });
	});
	await page.route('**/api/strategies/*/orders**', (route) =>
		route.fulfill({ json: [], headers: { 'content-type': 'application/json' } })
	);
	await page.route('**/api/strategies/*/decisions**', (route) =>
		route.fulfill({ json: [], headers: { 'content-type': 'application/json' } })
	);

	await page.goto('/live?strategy=smoke&exchange=binance&symbol=BTCUSDT&timeframe=1h&preset=24h');

	const selectors = page.getByTestId('live-selectors');
	if (!(await selectors.isVisible().catch(() => false))) {
		test.skip(true, 'viz backend not reachable — live selectors did not load');
	}

	const liveChart = page.getByTestId('live-chart');
	await expect(liveChart).toHaveAttribute('data-candle-count', String(CANDLES.length));

	// After ~10s the polling refetch fires and a new candle appears, untouched.
	await expect(liveChart).toHaveAttribute('data-candle-count', String(CANDLES.length + 1), {
		timeout: 15_000
	});
	// The poll was incremental: it asked for candles from the last known ts.
	expect(fromParams[1]).toBe(CANDLES[CANDLES.length - 1].ts);
});

test('shows a freshness indicator reflecting the last update (#27)', async ({ page }) => {
	await page.route('**/api/candles**', (route) =>
		route.fulfill({ json: CANDLES, headers: { 'content-type': 'application/json' } })
	);

	await page.goto('/live?strategy=smoke&exchange=binance&symbol=BTCUSDT&timeframe=1h&preset=24h');

	const selectors = page.getByTestId('live-selectors');
	if (!(await selectors.isVisible().catch(() => false))) {
		test.skip(true, 'viz backend not reachable — live selectors did not load');
	}

	await expect(page.getByTestId('chart').locator('canvas').first()).toBeVisible();
	const freshness = page.getByTestId('freshness-indicator');
	await expect(freshness).toBeVisible();
	// A fetch just succeeded, so the badge reads as fresh.
	await expect(freshness).toHaveAttribute('data-state', 'fresh');
});

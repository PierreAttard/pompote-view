import { page } from 'vitest/browser';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import LiveChart from './LiveChart.svelte';
import type { Candle, LiveDecision } from '$lib/api/types';

// Drive the candles render states by stubbing its query result. Orders/fills
// queries are best-effort annotations; they return empty data here so the focus
// stays on the candles-driven states. Decisions are overridable so we can drive
// the indicator toggles (#24). The mappers are covered by `annotations.spec.ts` /
// `indicators.spec.ts`.
const candlesResult = vi.hoisted(() => ({
	current: {} as { isPending: boolean; isError: boolean; data?: Candle[]; refetch: () => void }
}));
const decisionsData = vi.hoisted(() => ({ current: [] as LiveDecision[] }));

vi.mock('@tanstack/svelte-query', () => ({
	createQuery: (opts: () => { queryKey: unknown[] }) => {
		const resource = opts().queryKey[0];
		if (resource === 'candles') return candlesResult.current;
		if (resource === 'decisions')
			return { isPending: false, isError: false, data: decisionsData.current, refetch: vi.fn() };
		return { isPending: false, isError: false, data: [], refetch: vi.fn() };
	},
	// The candles queryFn reads prior data via the client; the mocked createQuery
	// ignores queryFn, so a no-op client is enough for the component to mount.
	useQueryClient: () => ({ getQueryData: () => undefined })
}));

beforeEach(() => {
	decisionsData.current = [];
});

const props = {
	strategyId: 'strat-1',
	exchange: 'binance',
	symbol: 'BTCUSDT',
	timeframe: '1h',
	from: '2026-06-01T00:00:00.000Z',
	to: '2026-06-02T00:00:00.000Z'
};

function candles(): Candle[] {
	return [
		{ ts: '2026-06-01T00:00:00Z', o: 100, h: 110, l: 95, c: 105, v: 12 },
		{ ts: '2026-06-01T01:00:00Z', o: 105, h: 120, l: 104, c: 118, v: 30 }
	];
}

describe('LiveChart.svelte', () => {
	it('shows the loading state while the query is pending', async () => {
		candlesResult.current = { isPending: true, isError: false, refetch: vi.fn() };
		render(LiveChart, props);
		await expect.element(page.getByTestId('live-chart-loading')).toBeInTheDocument();
	});

	it('shows an error with a working retry button', async () => {
		const refetch = vi.fn();
		candlesResult.current = { isPending: false, isError: true, refetch };
		render(LiveChart, props);
		await expect.element(page.getByTestId('live-chart-error')).toBeInTheDocument();
		await page.getByTestId('live-chart-retry').click();
		expect(refetch).toHaveBeenCalledOnce();
	});

	it('shows an empty state (and no volume profile) when the window has no candles', async () => {
		candlesResult.current = { isPending: false, isError: false, data: [], refetch: vi.fn() };
		render(LiveChart, props);
		await expect.element(page.getByTestId('live-chart-empty')).toBeInTheDocument();
		await expect.element(page.getByTestId('volume-profile')).not.toBeInTheDocument();
	});

	it('renders the chart and the volume profile when candles are available', async () => {
		candlesResult.current = { isPending: false, isError: false, data: candles(), refetch: vi.fn() };
		render(LiveChart, props);
		const chart = page.getByTestId('chart');
		await expect.element(chart).toBeInTheDocument();
		await expect.poll(() => chart.element().querySelectorAll('canvas').length).toBeGreaterThan(0);
		await expect.element(page.getByTestId('volume-profile')).toBeInTheDocument();
	});

	it('exposes indicator toggles and reconciles sub-chart panes when toggling', async () => {
		candlesResult.current = { isPending: false, isError: false, data: candles(), refetch: vi.fn() };
		decisionsData.current = [
			{
				decision_id: 'd1',
				session_id: 's1',
				created_at: '2026-06-01T00:00:00Z',
				reason: 'r',
				orders_count: 0,
				// `market_context` is opaque JSONB in the generated types; cast the sample.
				market_context: { adx: 30, atr: 1.2, rsi: 28 } as unknown as LiveDecision['market_context']
			}
		];
		render(LiveChart, props);

		await expect.element(page.getByTestId('live-indicator-toggles')).toBeInTheDocument();
		const adx = page.getByRole('button', { name: /adx/i });
		const atr = page.getByRole('button', { name: /atr/i });
		const rsi = page.getByRole('button', { name: /rsi/i });
		await expect.element(adx).toHaveAttribute('aria-pressed', 'false');

		// Enable all three → three sub-chart panes (must not throw).
		await adx.click();
		await atr.click();
		await rsi.click();
		await expect.element(atr).toHaveAttribute('aria-pressed', 'true');
		await expect
			.poll(() => page.getByTestId('chart').element().querySelectorAll('canvas').length)
			.toBeGreaterThan(0);

		// Disable the middle one → its pane is trimmed and the trailing one moves up.
		await atr.click();
		await expect.element(atr).toHaveAttribute('aria-pressed', 'false');
		await expect.element(adx).toHaveAttribute('aria-pressed', 'true');
		await expect.element(rsi).toHaveAttribute('aria-pressed', 'true');
		await expect
			.poll(() => page.getByTestId('chart').element().querySelectorAll('canvas').length)
			.toBeGreaterThan(0);
	});
});

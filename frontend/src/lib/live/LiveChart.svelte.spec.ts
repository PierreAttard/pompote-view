import { page } from 'vitest/browser';
import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import LiveChart from './LiveChart.svelte';
import type { Candle } from '$lib/api/types';

// Drive the candles render states by stubbing its query result. Orders/decisions
// queries are best-effort annotations; they return empty data here so the focus
// stays on the candles-driven states. The query wiring/mappers are covered by
// `candles.spec.ts` / `annotations.spec.ts`.
const candlesResult = vi.hoisted(() => ({
	current: {} as { isPending: boolean; isError: boolean; data?: Candle[]; refetch: () => void }
}));

vi.mock('@tanstack/svelte-query', () => ({
	createQuery: (opts: () => { queryKey: unknown[] }) => {
		const resource = opts().queryKey[0];
		if (resource === 'candles') return candlesResult.current;
		return { isPending: false, isError: false, data: [], refetch: vi.fn() };
	}
}));

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

	it('shows an empty state when the window has no candles', async () => {
		candlesResult.current = { isPending: false, isError: false, data: [], refetch: vi.fn() };
		render(LiveChart, props);
		await expect.element(page.getByTestId('live-chart-empty')).toBeInTheDocument();
	});

	it('renders the chart when candles are available', async () => {
		candlesResult.current = { isPending: false, isError: false, data: candles(), refetch: vi.fn() };
		render(LiveChart, props);
		const chart = page.getByTestId('chart');
		await expect.element(chart).toBeInTheDocument();
		await expect.poll(() => chart.element().querySelectorAll('canvas').length).toBeGreaterThan(0);
	});
});

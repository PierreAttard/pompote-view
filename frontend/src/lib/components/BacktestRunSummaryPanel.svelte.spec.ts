import { page } from 'vitest/browser';
import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import BacktestRunSummaryPanel from './BacktestRunSummaryPanel.svelte';
import type { BacktestRunSummary } from '$lib/api/types';

function run(overrides: Partial<BacktestRunSummary> = {}): BacktestRunSummary {
	return {
		id: '11111111-1111-1111-1111-111111111111',
		strategy_kind: 'directional',
		exchange: 'binance',
		symbol: 'BTCUSDT',
		data_range_start: '2026-06-01T00:00:00Z',
		data_range_end: '2026-06-01T03:00:00Z',
		started_at: '2026-06-01T09:00:00Z',
		status: 'completed',
		slippage_bps: 5,
		latency_ms: 200,
		fee_kraken_bps: 16,
		fee_binance_bps: 10,
		...overrides
	};
}

describe('BacktestRunSummaryPanel.svelte', () => {
	it('renders the run parameters', async () => {
		render(BacktestRunSummaryPanel, { run: run() });
		await expect.element(page.getByTestId('run-summary')).toBeInTheDocument();
		await expect.element(page.getByText('5 bps')).toBeInTheDocument();
		await expect.element(page.getByText('200 ms')).toBeInTheDocument();
		await expect.element(page.getByText('completed')).toBeInTheDocument();
	});

	it('shows the error message only for failed runs', async () => {
		render(BacktestRunSummaryPanel, { run: run({ status: 'failed', error: 'boom' }) });
		await expect.element(page.getByText('boom')).toBeInTheDocument();
	});
});

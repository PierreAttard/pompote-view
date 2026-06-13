import { page } from 'vitest/browser';
import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import BacktestRunsTable from './BacktestRunsTable.svelte';
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

describe('BacktestRunsTable.svelte', () => {
	it('renders one row per run with a link to its detail page', async () => {
		render(BacktestRunsTable, {
			runs: [run(), run({ id: '22222222-2222-2222-2222-222222222222', symbol: 'ETHUSDT' })]
		});

		await expect.element(page.getByTestId('backtests-table')).toBeInTheDocument();
		const links = page.getByTestId('run-link');
		await expect
			.element(links.first())
			.toHaveAttribute('href', '/backtests/11111111-1111-1111-1111-111111111111');
		await expect.element(page.getByText('BTCUSDT')).toBeInTheDocument();
		await expect.element(page.getByText('ETHUSDT')).toBeInTheDocument();
	});

	it('shows the status label', async () => {
		render(BacktestRunsTable, { runs: [run({ status: 'failed' })] });
		await expect.element(page.getByText('failed')).toBeInTheDocument();
	});

	it('shows an empty state when there are no runs', async () => {
		render(BacktestRunsTable, { runs: [] });
		await expect.element(page.getByTestId('backtests-empty')).toBeInTheDocument();
		await expect.element(page.getByTestId('backtests-table')).not.toBeInTheDocument();
	});
});

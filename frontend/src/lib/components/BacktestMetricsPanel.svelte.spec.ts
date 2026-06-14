import { page } from 'vitest/browser';
import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import BacktestMetricsPanel from './BacktestMetricsPanel.svelte';
import type { BacktestMetrics } from '$lib/api/types';

function metrics(overrides: Partial<BacktestMetrics> = {}): BacktestMetrics {
	return {
		buy_fills: 1,
		sell_fills: 1,
		buy_quantity: 0.5,
		sell_quantity: 0.5,
		buy_notional: 50,
		sell_notional: 65,
		total_fees: 0.3,
		gross_pnl: 15,
		net_pnl: 14.7,
		...overrides
	};
}

describe('BacktestMetricsPanel.svelte', () => {
	it('renders the recomputed metrics', async () => {
		render(BacktestMetricsPanel, { metrics: metrics() });
		await expect.element(page.getByTestId('run-metrics')).toBeInTheDocument();
		await expect.element(page.getByTestId('net-pnl')).toHaveTextContent('14,7');
		await expect.element(page.getByText('1 / 1')).toBeInTheDocument();
	});

	it('colours a positive net PnL green and a negative one red', async () => {
		const { rerender } = render(BacktestMetricsPanel, { metrics: metrics({ net_pnl: 10 }) });
		await expect.element(page.getByTestId('net-pnl')).toHaveClass(/emerald/);
		await rerender({ metrics: metrics({ net_pnl: -10 }) });
		await expect.element(page.getByTestId('net-pnl')).toHaveClass(/rose/);
	});
});

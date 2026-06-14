import { page } from 'vitest/browser';
import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import Chart from './Chart.svelte';
import type { Candle } from '$lib/api/types';

function candles(): Candle[] {
	return [
		{ ts: '2026-06-01T00:00:00Z', o: 100, h: 110, l: 95, c: 105, v: 12 },
		{ ts: '2026-06-01T01:00:00Z', o: 105, h: 120, l: 104, c: 118, v: 30 },
		{ ts: '2026-06-01T02:00:00Z', o: 118, h: 119, l: 100, c: 102, v: 21 }
	];
}

describe('Chart.svelte', () => {
	it('mounts a Lightweight Charts canvas', async () => {
		render(Chart, { candles: candles(), dark: true });
		const chart = page.getByTestId('chart');
		await expect.element(chart).toBeInTheDocument();
		// Lightweight Charts renders into <canvas> elements inside the container.
		await expect.poll(() => chart.element().querySelectorAll('canvas').length).toBeGreaterThan(0);
	});

	it('renders with an empty series without throwing', async () => {
		render(Chart, { candles: [], dark: false });
		await expect.element(page.getByTestId('chart')).toBeInTheDocument();
	});

	it('renders buy/sell markers without throwing', async () => {
		render(Chart, {
			candles: candles(),
			markers: [
				{ ts: '2026-06-01T00:00:00Z', side: 'buy', text: 'B' },
				{ ts: '2026-06-01T02:00:00Z', side: 'sell', text: 'S' }
			],
			dark: true
		});
		const chart = page.getByTestId('chart');
		await expect.element(chart).toBeInTheDocument();
		await expect.poll(() => chart.element().querySelectorAll('canvas').length).toBeGreaterThan(0);
	});

	it('renders indicator overlays without throwing', async () => {
		render(Chart, {
			candles: candles(),
			overlays: [
				{
					id: 'sma20',
					color: '#22c55e',
					title: 'SMA 20',
					data: [
						{ ts: '2026-06-01T00:00:00Z', value: 101 },
						{ ts: '2026-06-01T01:00:00Z', value: 110 },
						{ ts: '2026-06-01T02:00:00Z', value: 108 }
					]
				}
			],
			dark: true
		});
		const chart = page.getByTestId('chart');
		await expect.element(chart).toBeInTheDocument();
		await expect.poll(() => chart.element().querySelectorAll('canvas').length).toBeGreaterThan(0);
	});

	it('renders indicator sub-charts in their own panes without throwing', async () => {
		render(Chart, {
			candles: candles(),
			subcharts: [
				{
					id: 'rsi',
					color: '#3b82f6',
					title: 'RSI',
					data: [
						{ ts: '2026-06-01T00:00:00Z', value: 28 },
						{ ts: '2026-06-01T01:00:00Z', value: 35 },
						{ ts: '2026-06-01T02:00:00Z', value: 41 }
					]
				},
				{
					id: 'adx',
					color: '#f59e0b',
					title: 'ADX',
					data: [
						{ ts: '2026-06-01T00:00:00Z', value: 20 },
						{ ts: '2026-06-01T01:00:00Z', value: 22 },
						{ ts: '2026-06-01T02:00:00Z', value: 25 }
					]
				}
			],
			dark: true
		});
		const chart = page.getByTestId('chart');
		await expect.element(chart).toBeInTheDocument();
		await expect.poll(() => chart.element().querySelectorAll('canvas').length).toBeGreaterThan(0);
	});

	it('cleans up the chart DOM on unmount (no leak)', async () => {
		const { unmount } = render(Chart, { candles: candles() });
		const container = page.getByTestId('chart').element();
		await expect.poll(() => container.querySelectorAll('canvas').length).toBeGreaterThan(0);
		unmount();
		await expect.poll(() => container.querySelectorAll('canvas').length).toBe(0);
	});
});

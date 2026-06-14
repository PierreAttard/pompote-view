import { page } from 'vitest/browser';
import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import LiveSelectors from './LiveSelectors.svelte';
import type { Strategy } from '$lib/api/types';

const strategies: Strategy[] = [
	{ id: 'p1', name: 'paper_only', kind: 'directional', enabled_paper: true, enabled_live: false },
	{ id: 'l1', name: 'live_only', kind: 'meanrev', enabled_paper: false, enabled_live: true }
];

function props(overrides = {}) {
	return {
		strategies,
		timeframes: ['5s', '1m', '1h'],
		mode: 'all' as const,
		strategyId: '',
		exchange: 'binance',
		symbol: '',
		timeframe: '1h',
		preset: '24h' as const,
		onChange: vi.fn(),
		...overrides
	};
}

describe('LiveSelectors.svelte', () => {
	it('renders all selectors plus the mode filter', async () => {
		render(LiveSelectors, props());
		await expect.element(page.getByTestId('select-mode')).toBeInTheDocument();
		await expect.element(page.getByTestId('select-strategy')).toBeInTheDocument();
		await expect.element(page.getByTestId('select-exchange')).toBeInTheDocument();
		await expect.element(page.getByTestId('input-symbol')).toBeInTheDocument();
		await expect.element(page.getByTestId('select-timeframe')).toBeInTheDocument();
		await expect.element(page.getByTestId('preset-24h')).toBeInTheDocument();
	});

	it('reports an exchange change through onChange', async () => {
		const onChange = vi.fn();
		render(LiveSelectors, props({ onChange }));
		await page.getByTestId('select-exchange').selectOptions('kraken');
		expect(onChange).toHaveBeenCalledWith('exchange', 'kraken');
	});

	it('filters the strategy list to live-enabled when mode=live', async () => {
		render(LiveSelectors, props({ mode: 'live' }));
		// Scope to the primary select: a second (comparison) select also lists them.
		const primary = page.getByTestId('select-strategy');
		await expect.element(primary.getByRole('option', { name: /live_only/ })).toBeInTheDocument();
		expect(primary.getByRole('option', { name: /paper_only/ }).elements()).toHaveLength(0);
	});

	it('offers a comparison selector that excludes the primary strategy (#34)', async () => {
		render(LiveSelectors, props({ strategyId: 'l1' }));
		const compare = page.getByTestId('select-strategy-2');
		await expect.element(compare).toBeInTheDocument();
		await expect.element(compare.getByRole('option', { name: /Aucune/ })).toBeInTheDocument();
		// The other strategy is offered; the primary one is not (no self-compare).
		await expect.element(compare.getByRole('option', { name: /paper_only/ })).toBeInTheDocument();
		expect(compare.getByRole('option', { name: /live_only/ }).elements()).toHaveLength(0);
	});

	it('reports a comparison strategy change through onChange (#34)', async () => {
		const onChange = vi.fn();
		render(LiveSelectors, props({ strategyId: 'l1', onChange }));
		await page.getByTestId('select-strategy-2').selectOptions('p1');
		expect(onChange).toHaveBeenCalledWith('strategy2', 'p1');
	});

	it('reports a preset click through onChange', async () => {
		const onChange = vi.fn();
		render(LiveSelectors, props({ onChange }));
		await page.getByTestId('preset-1h').click();
		expect(onChange).toHaveBeenCalledWith('preset', '1h');
	});

	it('marks the active preset with aria-pressed', async () => {
		render(LiveSelectors, props({ preset: '6h' }));
		await expect.element(page.getByTestId('preset-6h')).toHaveAttribute('aria-pressed', 'true');
		await expect.element(page.getByTestId('preset-1h')).toHaveAttribute('aria-pressed', 'false');
	});
});

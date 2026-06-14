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
		symbol: '',
		timeframe: '1h',
		preset: '24h' as const,
		onChange: vi.fn(),
		...overrides
	};
}

describe('LiveSelectors.svelte', () => {
	it('renders all four selectors plus the mode filter', async () => {
		render(LiveSelectors, props());
		await expect.element(page.getByTestId('select-mode')).toBeInTheDocument();
		await expect.element(page.getByTestId('select-strategy')).toBeInTheDocument();
		await expect.element(page.getByTestId('input-symbol')).toBeInTheDocument();
		await expect.element(page.getByTestId('select-timeframe')).toBeInTheDocument();
		await expect.element(page.getByTestId('preset-24h')).toBeInTheDocument();
	});

	it('filters the strategy list to live-enabled when mode=live', async () => {
		render(LiveSelectors, props({ mode: 'live' }));
		await expect.element(page.getByRole('option', { name: /live_only/ })).toBeInTheDocument();
		expect(page.getByRole('option', { name: /paper_only/ }).elements()).toHaveLength(0);
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

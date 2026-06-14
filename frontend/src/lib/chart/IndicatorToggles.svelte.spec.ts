import { page } from 'vitest/browser';
import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import IndicatorToggles from './IndicatorToggles.svelte';

const indicators = [
	{ id: 'sma20', title: 'SMA 20', color: '#22c55e' },
	{ id: 'ema50', title: 'EMA 50', color: '#3b82f6' }
];

describe('IndicatorToggles.svelte', () => {
	it('renders one button per indicator with its label', async () => {
		render(IndicatorToggles, { indicators, visible: { sma20: true } });
		await expect.element(page.getByRole('button', { name: 'SMA 20' })).toBeInTheDocument();
		await expect.element(page.getByRole('button', { name: 'EMA 50' })).toBeInTheDocument();
	});

	it('reflects visibility through aria-pressed', async () => {
		render(IndicatorToggles, { indicators, visible: { sma20: true, ema50: false } });
		await expect
			.element(page.getByRole('button', { name: 'SMA 20' }))
			.toHaveAttribute('aria-pressed', 'true');
		await expect
			.element(page.getByRole('button', { name: 'EMA 50' }))
			.toHaveAttribute('aria-pressed', 'false');
	});

	it('toggles visibility on click', async () => {
		render(IndicatorToggles, { indicators, visible: { sma20: false } });
		const button = page.getByRole('button', { name: 'SMA 20' });
		await expect.element(button).toHaveAttribute('aria-pressed', 'false');
		await button.click();
		await expect.element(button).toHaveAttribute('aria-pressed', 'true');
	});
});

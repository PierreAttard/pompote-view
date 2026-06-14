import { page } from 'vitest/browser';
import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import FreshnessIndicator from './FreshnessIndicator.svelte';

describe('FreshnessIndicator.svelte', () => {
	it('shows a fresh state right after an update', async () => {
		render(FreshnessIndicator, { updatedAt: Date.now() });
		const el = page.getByTestId('freshness-indicator');
		await expect.element(el).toHaveAttribute('data-state', 'fresh');
		await expect.element(el).toHaveTextContent(/instant|il y a/);
	});

	it('turns stale past the threshold', async () => {
		render(FreshnessIndicator, { updatedAt: Date.now() - 40_000 });
		await expect
			.element(page.getByTestId('freshness-indicator'))
			.toHaveAttribute('data-state', 'stale');
	});

	it('shows an error state when the last poll failed', async () => {
		render(FreshnessIndicator, { updatedAt: Date.now(), errored: true });
		const el = page.getByTestId('freshness-indicator');
		await expect.element(el).toHaveAttribute('data-state', 'error');
		await expect.element(el).toHaveAttribute('aria-label', expect.stringContaining('Erreur'));
	});

	it('renders nothing before the first successful update', async () => {
		render(FreshnessIndicator, { updatedAt: 0 });
		await expect.element(page.getByTestId('freshness-indicator')).not.toBeInTheDocument();
	});
});

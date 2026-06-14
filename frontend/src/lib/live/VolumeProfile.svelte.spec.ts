import { page } from 'vitest/browser';
import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import VolumeProfile from './VolumeProfile.svelte';
import type { Candle } from '$lib/api/types';

/** Doji candle at a single price, so its volume lands in one bucket. */
function at(price: number, v: number): Candle {
	return { ts: '2026-06-01T00:00:00Z', o: price, h: price, l: price, c: price, v };
}

describe('VolumeProfile.svelte', () => {
	it('renders the profile with a POC and a value area', async () => {
		// Volumes per bucket [2, 8, 1, 6, 2] over [0, 5] → POC at bucket 1 (price 1.5).
		render(VolumeProfile, {
			candles: [at(0, 2), at(1, 8), at(2, 1), at(3, 6), at(5, 2)],
			bucketCount: 5
		});

		await expect.element(page.getByTestId('volume-profile')).toBeInTheDocument();
		await expect.element(page.getByTestId('vp-poc')).toHaveTextContent('POC 1.5');
		await expect.element(page.getByTestId('vp-value-area')).toHaveTextContent(/VA\s*0\s*[–-]\s*4/);
		await expect.poll(() => page.getByTestId('vp-bucket').elements().length).toBe(5);
		// HVN/LVN are identifiable beyond colour, via the bucket title (a11y). The
		// valley bucket (index 2) is the lone LVN of this distribution.
		await expect.element(page.getByTitle(/LVN/)).toBeInTheDocument();
	});

	it('exposes a descriptive aria-label summarising the POC and value area', async () => {
		render(VolumeProfile, { candles: [at(1, 8), at(2, 1), at(3, 6)], bucketCount: 3 });
		await expect
			.element(page.getByTestId('volume-profile'))
			.toHaveAttribute('aria-label', expect.stringContaining('POC'));
	});

	it('degrades cleanly when there is no volume', async () => {
		render(VolumeProfile, { candles: [] });
		await expect.element(page.getByText(/Pas de volume/)).toBeInTheDocument();
		await expect.element(page.getByTestId('vp-poc')).not.toBeInTheDocument();
	});
});

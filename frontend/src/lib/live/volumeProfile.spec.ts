import { describe, expect, it } from 'vitest';
import type { Candle } from '$lib/api/types';
import { computeVolumeProfile } from './volumeProfile';

/** Doji candle at a single price (l === h), so its volume lands in one bucket. */
function at(price: number, v: number): Candle {
	return { ts: '2026-06-01T00:00:00Z', o: price, h: price, l: price, c: price, v };
}

describe('computeVolumeProfile', () => {
	it('returns an empty profile for no candles', () => {
		const p = computeVolumeProfile([]);
		expect(p.buckets).toEqual([]);
		expect(p.pocIndex).toBe(-1);
		expect(p.totalVolume).toBe(0);
	});

	it('returns an empty profile when there is no volume', () => {
		expect(computeVolumeProfile([at(1, 0), at(2, 0)]).pocIndex).toBe(-1);
	});

	it('collapses to a single bucket for a flat price range', () => {
		const p = computeVolumeProfile([at(100, 5), at(100, 5)]);
		expect(p.buckets).toHaveLength(1);
		expect(p.buckets[0]).toMatchObject({ price: 100, volume: 10 });
		expect(p.pocIndex).toBe(0);
		expect(p.valueAreaLow).toBe(0);
		expect(p.valueAreaHigh).toBe(0);
		expect(p.totalVolume).toBe(10);
	});

	it('locates the POC, value area and HVN/LVN of a distribution', () => {
		// Dojis at integer prices 0..5 with 5 buckets of width 1 → volumes per
		// bucket [2, 8, 1, 6, 2]. Anchors at 0 and 5 pin the [0, 5] range.
		const p = computeVolumeProfile([at(0, 2), at(1, 8), at(2, 1), at(3, 6), at(5, 2)], 5);
		expect(p.buckets.map((b) => Math.round(b.volume))).toEqual([2, 8, 1, 6, 2]);
		expect(p.pocIndex).toBe(1); // heaviest bucket
		expect(p.maxVolume).toBe(8);
		expect(p.totalVolume).toBe(19);
		// Greedy 70% value area grown from the POC.
		expect([p.valueAreaLow, p.valueAreaHigh]).toEqual([0, 3]);
		// Local maxima / minima (interior buckets).
		expect(p.hvnIndices).toEqual([1, 3]);
		expect(p.lvnIndices).toEqual([2]);
	});

	it('spreads a wide candle proportionally across the buckets it spans', () => {
		// One candle spanning [0, 2] with volume 10, two buckets → 5 each.
		const candle: Candle = { ts: '2026-06-01T00:00:00Z', o: 0, h: 2, l: 0, c: 2, v: 10 };
		const p = computeVolumeProfile([candle], 2);
		expect(p.buckets.map((b) => b.volume)).toEqual([5, 5]);
		expect(p.totalVolume).toBe(10);
	});

	it('keeps the value area within the bucket bounds', () => {
		const p = computeVolumeProfile([at(0, 1), at(1, 1), at(2, 1), at(3, 1), at(5, 1)], 5);
		expect(p.valueAreaLow).toBeGreaterThanOrEqual(0);
		expect(p.valueAreaHigh).toBeLessThanOrEqual(p.buckets.length - 1);
		expect(p.valueAreaLow).toBeLessThanOrEqual(p.pocIndex);
		expect(p.valueAreaHigh).toBeGreaterThanOrEqual(p.pocIndex);
	});
});

/**
 * Volume Profile (#25): aggregates traded volume by price level from the OHLCV
 * candles already loaded for the live chart — no extra backend call, no
 * `robot_rust` dependency.
 *
 * V1 approximation (documented; `robot_rust` issue #624 owns the canonical
 * definition, not available here):
 * - Each candle's volume is spread **uniformly over its [low, high] range**,
 *   proportionally to the overlap with each price bucket. A flat candle drops
 *   its whole volume in the bucket containing its price.
 * - **POC** (Point of Control) = bucket with the most volume.
 * - **Value Area** = the contiguous band around the POC holding ≥70% of the
 *   total volume, grown greedily towards the heavier adjacent bucket.
 * - **HVN / LVN** = interior buckets that are strict local maxima / minima.
 *
 * Pure and DOM-free so it is unit-testable in the node project.
 */
import type { Candle } from '$lib/api/types';

/** One price bucket of the profile. */
export interface VolumeBucket {
	/** Inclusive lower price bound. */
	low: number;
	/** Upper price bound (exclusive, except for the topmost bucket). */
	high: number;
	/** Bucket mid-price (for labels). */
	price: number;
	/** Volume aggregated into this bucket. */
	volume: number;
}

/** Computed volume profile, buckets ordered from lowest to highest price. */
export interface VolumeProfile {
	buckets: VolumeBucket[];
	/** Index of the Point-of-Control bucket, or -1 when empty. */
	pocIndex: number;
	/** Inclusive bucket index range of the value area, or -1 when empty. */
	valueAreaLow: number;
	valueAreaHigh: number;
	/** Largest single-bucket volume (for bar scaling). */
	maxVolume: number;
	totalVolume: number;
	/** Interior buckets that are strict local maxima / minima. */
	hvnIndices: number[];
	lvnIndices: number[];
}

/** Fraction of total volume the value area must cover. */
const VALUE_AREA_RATIO = 0.7;

const EMPTY: VolumeProfile = {
	buckets: [],
	pocIndex: -1,
	valueAreaLow: -1,
	valueAreaHigh: -1,
	maxVolume: 0,
	totalVolume: 0,
	hvnIndices: [],
	lvnIndices: []
};

/**
 * Builds the volume profile for the given candles. Returns an empty profile when
 * there are no candles or no volume (clean degradation).
 *
 * `bucketCount` is the number of price levels (default 24).
 */
export function computeVolumeProfile(candles: Candle[], bucketCount = 24): VolumeProfile {
	if (candles.length === 0 || bucketCount < 1) return EMPTY;

	let priceMin = Infinity;
	let priceMax = -Infinity;
	for (const c of candles) {
		if (c.l < priceMin) priceMin = c.l;
		if (c.h > priceMax) priceMax = c.h;
	}
	if (!Number.isFinite(priceMin) || !Number.isFinite(priceMax)) return EMPTY;

	// Degenerate flat range: a single bucket holding the whole volume.
	if (priceMax === priceMin) {
		const volume = candles.reduce((s, c) => s + (Number.isFinite(c.v) ? c.v : 0), 0);
		if (volume <= 0) return EMPTY;
		return {
			buckets: [{ low: priceMin, high: priceMax, price: priceMin, volume }],
			pocIndex: 0,
			valueAreaLow: 0,
			valueAreaHigh: 0,
			maxVolume: volume,
			totalVolume: volume,
			hvnIndices: [],
			lvnIndices: []
		};
	}

	const n = bucketCount;
	const size = (priceMax - priceMin) / n;
	const buckets: VolumeBucket[] = Array.from({ length: n }, (_, i) => {
		const low = priceMin + i * size;
		return { low, high: low + size, price: low + size / 2, volume: 0 };
	});

	const indexOf = (price: number) =>
		Math.min(n - 1, Math.max(0, Math.floor((price - priceMin) / size)));

	for (const c of candles) {
		const v = Number.isFinite(c.v) ? c.v : 0;
		if (v <= 0) continue;
		const lo = Math.min(c.l, c.h);
		const hi = Math.max(c.l, c.h);
		if (hi === lo) {
			buckets[indexOf(lo)].volume += v;
			continue;
		}
		// Spread proportionally to the overlap of [lo, hi] with each touched bucket.
		const span = hi - lo;
		for (let i = indexOf(lo); i <= indexOf(hi); i++) {
			const overlap = Math.min(hi, buckets[i].high) - Math.max(lo, buckets[i].low);
			if (overlap > 0) buckets[i].volume += v * (overlap / span);
		}
	}

	let totalVolume = 0;
	let pocIndex = 0;
	let maxVolume = 0;
	for (let i = 0; i < n; i++) {
		totalVolume += buckets[i].volume;
		if (buckets[i].volume > maxVolume) {
			maxVolume = buckets[i].volume;
			pocIndex = i;
		}
	}
	if (totalVolume <= 0) return EMPTY;

	// Value area: grow from the POC towards the heavier neighbour until ≥70%.
	const target = VALUE_AREA_RATIO * totalVolume;
	let lo = pocIndex;
	let hi = pocIndex;
	let vaVolume = buckets[pocIndex].volume;
	while (vaVolume < target && (lo > 0 || hi < n - 1)) {
		const below = lo > 0 ? buckets[lo - 1].volume : -1;
		const above = hi < n - 1 ? buckets[hi + 1].volume : -1;
		if (above >= below) {
			hi += 1;
			vaVolume += buckets[hi].volume;
		} else {
			lo -= 1;
			vaVolume += buckets[lo].volume;
		}
	}

	const hvnIndices: number[] = [];
	const lvnIndices: number[] = [];
	for (let i = 1; i < n - 1; i++) {
		const v = buckets[i].volume;
		if (v > buckets[i - 1].volume && v > buckets[i + 1].volume) hvnIndices.push(i);
		else if (v < buckets[i - 1].volume && v < buckets[i + 1].volume) lvnIndices.push(i);
	}

	return {
		buckets,
		pocIndex,
		valueAreaLow: lo,
		valueAreaHigh: hi,
		maxVolume,
		totalVolume,
		hvnIndices,
		lvnIndices
	};
}

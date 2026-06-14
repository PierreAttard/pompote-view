/**
 * UI-side depth guard for the live candles query.
 *
 * The backend caps a candles response at `MAX_CANDLE_POINTS` buckets and rejects
 * anything larger with HTTP 400. Computing the same estimate here lets us give
 * immediate feedback and, crucially, skip firing a request that is guaranteed to
 * fail (issue #19). The maths mirrors the backend's `estimate_bucket_count`:
 * ceiling division of the window (in seconds) by the timeframe width.
 *
 * Kept Svelte/DOM-free so the validation is unit-testable in isolation.
 */
import { presetMs, type RangePreset } from './range';

/** Hard cap on the number of buckets, mirroring `domain::MAX_CANDLE_POINTS`. */
export const MAX_CANDLE_POINTS = 5000;

/** Seconds in each timeframe unit (`5s`, `1m`, `1h`, `1d`). */
const UNIT_SECONDS: Record<string, number> = {
	s: 1,
	m: 60,
	h: 60 * 60,
	d: 24 * 60 * 60
};

/**
 * Parses a timeframe id (`<n><unit>`, e.g. `15m`, `4h`) into seconds, or returns
 * `null` for an unrecognised shape. Mirrors the backend's allowed set, whose ids
 * all match this `number + s|m|h|d` pattern.
 */
export function timeframeSeconds(timeframe: string): number | null {
	const match = /^(\d+)(s|m|h|d)$/.exec(timeframe);
	if (!match) return null;
	const n = Number(match[1]);
	if (n <= 0) return null;
	return n * UNIT_SECONDS[match[2]];
}

/**
 * Number of buckets a `(preset, timeframe)` pair would request, using ceiling
 * division like the backend. Returns `null` when the timeframe is unparseable
 * (we then can't validate and don't block).
 */
export function bucketCount(preset: RangePreset, timeframe: string): number | null {
	const width = timeframeSeconds(timeframe);
	if (width === null) return null;
	const rangeSeconds = presetMs(preset) / 1000;
	return Math.ceil(rangeSeconds / width);
}

/**
 * Finest timeframe from `timeframes` (assumed ordered finest → coarsest, as the
 * backend returns them) that keeps the window at or under the cap, or `null`
 * when even the coarsest one overflows (the range itself is too wide).
 */
export function suggestTimeframe(preset: RangePreset, timeframes: string[]): string | null {
	for (const tf of timeframes) {
		const count = bucketCount(preset, tf);
		if (count !== null && count <= MAX_CANDLE_POINTS) return tf;
	}
	return null;
}

/** Outcome of the depth check for a `(preset, timeframe)` selection. */
export interface DepthStatus {
	/** Estimated bucket count, or `null` when the timeframe is unparseable. */
	count: number | null;
	/** `true` when the selection would exceed the backend cap. */
	exceeds: boolean;
	/** Finest timeframe that fits, or `null` when no timeframe does. */
	suggestion: string | null;
}

/**
 * Full depth verdict for a selection: the estimated count, whether it exceeds
 * the cap, and the finest timeframe that would fit.
 */
export function depthStatus(
	preset: RangePreset,
	timeframe: string,
	timeframes: string[]
): DepthStatus {
	const count = bucketCount(preset, timeframe);
	const exceeds = count !== null && count > MAX_CANDLE_POINTS;
	return {
		count,
		exceeds,
		suggestion: exceeds ? suggestTimeframe(preset, timeframes) : null
	};
}

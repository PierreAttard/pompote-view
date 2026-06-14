/**
 * Pure helpers for the `<Chart>` wrapper, kept out of the component so they can
 * be unit-tested without the browser/canvas machinery of Lightweight Charts.
 */
import type { CandlestickData, UTCTimestamp } from 'lightweight-charts';
import type { Candle } from '$lib/api/types';

/**
 * Convert API candles into Lightweight Charts series data.
 *
 * The library requires strictly ascending, de-duplicated timestamps in seconds.
 * The API returns RFC3339 strings on `ts`, so we parse, drop unparseable rows,
 * sort, and keep the last occurrence of any duplicated bucket.
 */
export function toSeriesData(input: Candle[]): CandlestickData[] {
	const points: CandlestickData[] = [];
	for (const c of input) {
		const time = Math.floor(Date.parse(c.ts) / 1000);
		if (Number.isNaN(time)) continue;
		points.push({ time: time as UTCTimestamp, open: c.o, high: c.h, low: c.l, close: c.c });
	}
	points.sort((a, b) => (a.time as number) - (b.time as number));
	// Drop duplicate buckets, keeping the last occurrence for each timestamp.
	return points.filter((p, i) => i === points.length - 1 || p.time !== points[i + 1].time);
}

/**
 * Whether `next` extends `prev` so the chart can grow with `series.update()`
 * (re-applying the boundary bar + appending new ones) instead of a full
 * `setData()` (#26 live polling).
 *
 * Safe because candle history is immutable — only the latest bar mutates and new
 * bars append. Requires the same first bucket (same window start) and a
 * non-shrinking length; a different start (selection change, cap-trim) falls back
 * to `setData`.
 */
export function isIncrementalAppend(prev: CandlestickData[], next: CandlestickData[]): boolean {
	return prev.length > 0 && next.length >= prev.length && next[0]?.time === prev[0]?.time;
}

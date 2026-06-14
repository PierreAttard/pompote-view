/**
 * Pure helpers + types for indicator overlays (SMA/EMA/Bollinger…), kept out of
 * the component so they can be unit-tested without the browser/canvas machinery
 * of Lightweight Charts. Source-agnostic: live and backtest views build the
 * same `ChartOverlay` shape from their own data.
 */
import type { LineData, UTCTimestamp } from 'lightweight-charts';

/** One value of an indicator line at a given timestamp. */
export interface OverlayPoint {
	/** RFC3339 timestamp (`market_ts` for backtest, candle `ts` for live). */
	ts: string;
	/** Indicator value; `null`/non-finite points are dropped (gaps). */
	value: number | null;
}

/** A single indicator line drawn on top of the price series. */
export interface ChartOverlay {
	/** Stable key used to add/update/remove the line (e.g. `sma20`). */
	id: string;
	/** Line colour. */
	color: string;
	/** Legend label shown on the chart (defaults to `id`). */
	title?: string;
	/** Ordered-or-not points; sorted and cleaned before rendering. */
	data: OverlayPoint[];
}

/**
 * Convert overlay points into Lightweight Charts line data.
 *
 * Drops points with an unparseable timestamp or a non-finite value, sorts by
 * ascending time, and keeps the last value for duplicated timestamps (the
 * library requires ascending, de-duplicated data).
 */
export function toLineData(input: OverlayPoint[]): LineData<UTCTimestamp>[] {
	const points: LineData<UTCTimestamp>[] = [];
	for (const p of input) {
		const time = Math.floor(Date.parse(p.ts) / 1000);
		if (Number.isNaN(time)) continue;
		if (p.value === null || !Number.isFinite(p.value)) continue;
		points.push({ time: time as UTCTimestamp, value: p.value });
	}
	points.sort((a, b) => (a.time as number) - (b.time as number));
	return points.filter((p, i) => i === points.length - 1 || p.time !== points[i + 1].time);
}

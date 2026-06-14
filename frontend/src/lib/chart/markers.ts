/**
 * Pure helpers for decision/order markers, kept out of the component so they
 * can be unit-tested without the browser/canvas machinery of Lightweight
 * Charts. Source-agnostic: live and backtest views map their rows to the same
 * `ChartMarker` shape.
 */
import type { SeriesMarker, UTCTimestamp } from 'lightweight-charts';

/** A buy/sell annotation to place on the chart at a given timestamp. */
export interface ChartMarker {
	/** RFC3339 timestamp (`market_ts` for backtest, candle `ts` for live). */
	ts: string;
	/** Order/decision side; `buy`/`sell` get dedicated styling, others stay neutral. */
	side: string;
	/** Optional short label drawn next to the marker (e.g. `B`/`S`). */
	text?: string;
	/**
	 * Optional colour override. The shape still encodes the side (up/down arrow);
	 * this lets a second compared strategy use a distinct colour (#34).
	 */
	color?: string;
}

const BUY_COLOR = '#22c55e';
const SELL_COLOR = '#ef4444';
const NEUTRAL_COLOR = '#94a3b8';

/**
 * Convert source-agnostic markers into Lightweight Charts series markers.
 *
 * Buy → green up-arrow below the bar; sell → red down-arrow above the bar;
 * anything else → neutral circle, so unexpected sides stay visible instead of
 * silently vanishing. Markers must be in ascending time order, so we parse,
 * drop unparseable rows, and sort before returning.
 */
export function toSeriesMarkers(input: ChartMarker[]): SeriesMarker<UTCTimestamp>[] {
	const markers: SeriesMarker<UTCTimestamp>[] = [];
	for (const m of input) {
		const time = Math.floor(Date.parse(m.ts) / 1000);
		if (Number.isNaN(time)) continue;
		// Defensive: API rows can contradict the `string` type at runtime.
		const side = String(m.side ?? '').toLowerCase();
		if (side === 'buy') {
			markers.push({
				time: time as UTCTimestamp,
				position: 'belowBar',
				shape: 'arrowUp',
				color: m.color ?? BUY_COLOR,
				text: m.text
			});
		} else if (side === 'sell') {
			markers.push({
				time: time as UTCTimestamp,
				position: 'aboveBar',
				shape: 'arrowDown',
				color: m.color ?? SELL_COLOR,
				text: m.text
			});
		} else {
			markers.push({
				time: time as UTCTimestamp,
				position: 'inBar',
				shape: 'circle',
				color: m.color ?? NEUTRAL_COLOR,
				text: m.text
			});
		}
	}
	return markers.sort((a, b) => (a.time as number) - (b.time as number));
}

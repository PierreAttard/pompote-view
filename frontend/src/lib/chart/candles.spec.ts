import { describe, expect, it } from 'vitest';
import type { CandlestickData, UTCTimestamp } from 'lightweight-charts';
import { isIncrementalAppend, toSeriesData } from './candles';
import type { Candle } from '$lib/api/types';

function bar(time: number): CandlestickData {
	return { time: time as UTCTimestamp, open: 1, high: 2, low: 0.5, close: 1.5 };
}

function candle(ts: string, o = 1, h = 2, l = 0.5, c = 1.5, v = 10): Candle {
	return { ts, o, h, l, c, v };
}

describe('toSeriesData', () => {
	it('maps OHLC fields and converts ts to unix seconds', () => {
		const [point] = toSeriesData([candle('2026-06-01T00:00:00Z', 100, 110, 95, 105)]);
		expect(point).toEqual({
			time: Date.parse('2026-06-01T00:00:00Z') / 1000,
			open: 100,
			high: 110,
			low: 95,
			close: 105
		});
	});

	it('sorts points by ascending time', () => {
		const data = toSeriesData([
			candle('2026-06-01T02:00:00Z'),
			candle('2026-06-01T00:00:00Z'),
			candle('2026-06-01T01:00:00Z')
		]);
		const times = data.map((p) => p.time as number);
		expect(times).toEqual([...times].sort((a, b) => a - b));
		expect(times).toHaveLength(3);
	});

	it('drops duplicate buckets, keeping the last occurrence', () => {
		const data = toSeriesData([
			candle('2026-06-01T00:00:00Z', 1, 1, 1, 1),
			candle('2026-06-01T00:00:00Z', 9, 9, 9, 9)
		]);
		expect(data).toHaveLength(1);
		expect(data[0]).toMatchObject({ open: 9, close: 9 });
	});

	it('skips rows with an unparseable timestamp', () => {
		const data = toSeriesData([candle('not-a-date'), candle('2026-06-01T00:00:00Z')]);
		expect(data).toHaveLength(1);
	});

	it('returns an empty array for empty input', () => {
		expect(toSeriesData([])).toEqual([]);
	});
});

describe('isIncrementalAppend', () => {
	it('is true when the tail grows from the same window start', () => {
		expect(isIncrementalAppend([bar(1), bar(2)], [bar(1), bar(2), bar(3)])).toBe(true);
	});

	it('is true when only the latest bar mutates (same length, same start)', () => {
		expect(isIncrementalAppend([bar(1), bar(2)], [bar(1), bar(2)])).toBe(true);
	});

	it('is false on the first render (no previous data)', () => {
		expect(isIncrementalAppend([], [bar(1)])).toBe(false);
	});

	it('is false when the window start changed (selection change / cap-trim)', () => {
		expect(isIncrementalAppend([bar(1), bar(2)], [bar(2), bar(3)])).toBe(false);
	});

	it('is false when the series shrank', () => {
		expect(isIncrementalAppend([bar(1), bar(2), bar(3)], [bar(1), bar(2)])).toBe(false);
	});
});

import { describe, expect, it } from 'vitest';
import { toSeriesData } from './candles';
import type { Candle } from '$lib/api/types';

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

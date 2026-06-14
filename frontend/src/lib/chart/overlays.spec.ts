import { describe, expect, it } from 'vitest';
import { toLineData, type OverlayPoint } from './overlays';

function point(ts: string, value: number | null): OverlayPoint {
	return { ts, value };
}

describe('toLineData', () => {
	it('maps ts to unix seconds and keeps the value', () => {
		const [p] = toLineData([point('2026-06-01T00:00:00Z', 42)]);
		expect(p).toEqual({ time: Date.parse('2026-06-01T00:00:00Z') / 1000, value: 42 });
	});

	it('sorts points by ascending time', () => {
		const data = toLineData([
			point('2026-06-01T02:00:00Z', 3),
			point('2026-06-01T00:00:00Z', 1),
			point('2026-06-01T01:00:00Z', 2)
		]);
		expect(data.map((p) => p.value)).toEqual([1, 2, 3]);
	});

	it('keeps the last value for duplicate timestamps', () => {
		const data = toLineData([point('2026-06-01T00:00:00Z', 1), point('2026-06-01T00:00:00Z', 9)]);
		expect(data).toHaveLength(1);
		expect(data[0].value).toBe(9);
	});

	it('drops null and non-finite values (gaps)', () => {
		const data = toLineData([
			point('2026-06-01T00:00:00Z', null),
			point('2026-06-01T01:00:00Z', Number.NaN),
			point('2026-06-01T02:00:00Z', Number.POSITIVE_INFINITY),
			point('2026-06-01T03:00:00Z', 5)
		]);
		expect(data).toHaveLength(1);
		expect(data[0].value).toBe(5);
	});

	it('skips rows with an unparseable timestamp', () => {
		expect(toLineData([point('nope', 1)])).toHaveLength(0);
	});

	it('returns an empty array for empty input', () => {
		expect(toLineData([])).toEqual([]);
	});
});

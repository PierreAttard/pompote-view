import { describe, expect, it } from 'vitest';
import { toSeriesMarkers, type ChartMarker } from './markers';

function marker(ts: string, side: string, text?: string): ChartMarker {
	return { ts, side, text };
}

describe('toSeriesMarkers', () => {
	it('maps buy to a green up-arrow below the bar', () => {
		const [m] = toSeriesMarkers([marker('2026-06-01T00:00:00Z', 'buy', 'B')]);
		expect(m).toMatchObject({ position: 'belowBar', shape: 'arrowUp', text: 'B' });
		expect(m.color).toBe('#22c55e');
		expect(m.time).toBe(Date.parse('2026-06-01T00:00:00Z') / 1000);
	});

	it('maps sell to a red down-arrow above the bar', () => {
		const [m] = toSeriesMarkers([marker('2026-06-01T00:00:00Z', 'sell', 'S')]);
		expect(m).toMatchObject({ position: 'aboveBar', shape: 'arrowDown', text: 'S' });
		expect(m.color).toBe('#ef4444');
	});

	it('is case-insensitive on side', () => {
		const [m] = toSeriesMarkers([marker('2026-06-01T00:00:00Z', 'BUY')]);
		expect(m.shape).toBe('arrowUp');
	});

	it('renders an unknown side as a neutral circle rather than dropping it', () => {
		const [m] = toSeriesMarkers([marker('2026-06-01T00:00:00Z', 'hold')]);
		expect(m).toMatchObject({ position: 'inBar', shape: 'circle' });
	});

	it('skips rows with an unparseable timestamp', () => {
		const data = toSeriesMarkers([marker('nope', 'buy'), marker('2026-06-01T00:00:00Z', 'sell')]);
		expect(data).toHaveLength(1);
		expect(data[0].shape).toBe('arrowDown');
	});

	it('sorts markers by ascending time', () => {
		const data = toSeriesMarkers([
			marker('2026-06-01T02:00:00Z', 'buy'),
			marker('2026-06-01T00:00:00Z', 'sell'),
			marker('2026-06-01T01:00:00Z', 'buy')
		]);
		const times = data.map((m) => m.time as number);
		expect(times).toEqual([...times].sort((a, b) => a - b));
	});

	it('returns an empty array for empty input', () => {
		expect(toSeriesMarkers([])).toEqual([]);
	});

	it('honours a colour override while keeping the side-driven shape (#34)', () => {
		const [buy] = toSeriesMarkers([{ ts: '2026-06-01T00:00:00Z', side: 'buy', color: '#a855f7' }]);
		expect(buy).toMatchObject({ shape: 'arrowUp', color: '#a855f7' });
		const [sell] = toSeriesMarkers([
			{ ts: '2026-06-01T00:00:00Z', side: 'sell', color: '#a855f7' }
		]);
		expect(sell).toMatchObject({ shape: 'arrowDown', color: '#a855f7' });
	});
});

import { describe, expect, it } from 'vitest';
import { ordersToMarkers, decisionsToOverlays } from './series';
import type { BacktestOrder, Decision } from '$lib/api/types';

function order(side: string, market_ts = '2026-06-01T00:00:00Z'): BacktestOrder {
	return {
		decision_id: 'd',
		order_id: 'o',
		market_ts,
		quantity: 1,
		side,
		status: 'filled'
	} as BacktestOrder;
}

function decision(market_ts: string, snapshot: Record<string, unknown> | null): Decision {
	return {
		decision_id: 'd',
		market_ts,
		orders_count: 0,
		reason: 'r',
		snapshot
	} as unknown as Decision;
}

describe('ordersToMarkers', () => {
	it('labels buy as B and sell as S, preserving side and ts', () => {
		const markers = ordersToMarkers([order('buy', '2026-06-01T00:00:00Z'), order('sell')]);
		expect(markers[0]).toMatchObject({ side: 'buy', text: 'B', ts: '2026-06-01T00:00:00Z' });
		expect(markers[1]).toMatchObject({ side: 'sell', text: 'S' });
	});

	it('falls back to ? for an unexpected side', () => {
		expect(ordersToMarkers([order('hold')])[0].text).toBe('?');
	});

	it('returns an empty array for no orders', () => {
		expect(ordersToMarkers([])).toEqual([]);
	});
});

describe('decisionsToOverlays', () => {
	it('builds one overlay line per numeric snapshot field, sorted by key', () => {
		const overlays = decisionsToOverlays([
			decision('2026-06-01T00:00:00Z', { ema50: 9, sma20: 1 }),
			decision('2026-06-01T01:00:00Z', { ema50: 11, sma20: 2 })
		]);
		expect(overlays.map((o) => o.id)).toEqual(['ema50', 'sma20']);
		expect(overlays[0].data).toEqual([
			{ ts: '2026-06-01T00:00:00Z', value: 9 },
			{ ts: '2026-06-01T01:00:00Z', value: 11 }
		]);
		expect(overlays[0].color).toBeTruthy();
	});

	it('ignores non-numeric and non-finite fields', () => {
		const overlays = decisionsToOverlays([
			decision('2026-06-01T00:00:00Z', { sma20: 5, label: 'x', nan: Number.NaN })
		]);
		expect(overlays.map((o) => o.id)).toEqual(['sma20']);
	});

	it('yields no overlays for null/empty snapshots (clean degradation)', () => {
		expect(decisionsToOverlays([decision('2026-06-01T00:00:00Z', null)])).toEqual([]);
		expect(decisionsToOverlays([decision('2026-06-01T00:00:00Z', {})])).toEqual([]);
		expect(decisionsToOverlays([])).toEqual([]);
	});
});

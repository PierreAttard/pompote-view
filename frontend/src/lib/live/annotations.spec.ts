import { describe, expect, it, vi } from 'vitest';
import type { LiveDecision, Order } from '$lib/api/types';
import {
	bucketStartSeconds,
	buildDecisionLookup,
	decisionsQueryKey,
	fetchOrders,
	indexDecisionsById,
	ordersQueryKey,
	ordersToMarkers
} from './annotations';

function order(overrides: Partial<Order> = {}): Order {
	return {
		created_at: '2026-06-01T00:30:00Z',
		decision_id: 'd1',
		order_id: 'o1',
		price: 100,
		quantity: 1,
		side: 'buy',
		status: 'filled',
		...overrides
	};
}

function decision(id: string, overrides: Partial<LiveDecision> = {}): LiveDecision {
	return {
		decision_id: id,
		created_at: '2026-06-01T00:30:00Z',
		reason: `reason ${id}`,
		orders_count: 1,
		session_id: 's1',
		...overrides
	};
}

describe('ordersToMarkers', () => {
	it('maps side to B/S/? labels and uses created_at as the timestamp', () => {
		const markers = ordersToMarkers([
			order({ side: 'buy', created_at: '2026-06-01T00:00:00Z' }),
			order({ side: 'sell' }),
			order({ side: 'weird' as string })
		]);
		expect(markers).toEqual([
			{ ts: '2026-06-01T00:00:00Z', side: 'buy', text: 'B' },
			{ ts: '2026-06-01T00:30:00Z', side: 'sell', text: 'S' },
			{ ts: '2026-06-01T00:30:00Z', side: 'weird', text: '?' }
		]);
	});
});

describe('indexDecisionsById', () => {
	it('keys decisions by their id', () => {
		const idx = indexDecisionsById([decision('a'), decision('b')]);
		expect(idx.get('a')?.reason).toBe('reason a');
		expect(idx.size).toBe(2);
	});
});

describe('bucketStartSeconds', () => {
	it('floors a timestamp to its candle-bucket start', () => {
		// 00:30 with a 1h timeframe falls in the 00:00 bucket.
		const at0030 = bucketStartSeconds('2026-06-01T00:30:00Z', 3600);
		const at0000 = bucketStartSeconds('2026-06-01T00:00:00Z', 3600);
		expect(at0030).toBe(at0000);
		expect(at0000).toBe(Date.parse('2026-06-01T00:00:00Z') / 1000);
	});

	it('returns null for an unparseable timestamp or non-positive width', () => {
		expect(bucketStartSeconds('nope', 3600)).toBeNull();
		expect(bucketStartSeconds('2026-06-01T00:00:00Z', 0)).toBeNull();
	});
});

describe('buildDecisionLookup', () => {
	it('groups decisions by candle bucket via order.decision_id, de-duplicating', () => {
		const orders = [
			order({ decision_id: 'd1', created_at: '2026-06-01T00:30:00Z' }),
			order({ decision_id: 'd1', created_at: '2026-06-01T00:45:00Z' }),
			order({ decision_id: 'd2', created_at: '2026-06-01T01:10:00Z' })
		];
		const lookup = buildDecisionLookup(orders, [decision('d1'), decision('d2')], 3600);

		const bucket0 = bucketStartSeconds('2026-06-01T00:00:00Z', 3600)!;
		const bucket1 = bucketStartSeconds('2026-06-01T01:00:00Z', 3600)!;
		expect(lookup.get(bucket0)?.map((d) => d.decision_id)).toEqual(['d1']);
		expect(lookup.get(bucket1)?.map((d) => d.decision_id)).toEqual(['d2']);
	});

	it('skips orders whose decision is absent', () => {
		const lookup = buildDecisionLookup([order({ decision_id: 'ghost' })], [decision('d1')], 3600);
		expect(lookup.size).toBe(0);
	});
});

describe('query keys + fetchers', () => {
	it('builds distinct query keys per resource and window', () => {
		const p = { strategyId: 's1', from: 'F', to: 'T' };
		expect(ordersQueryKey(p)).toEqual(['orders', 's1', 'F', 'T']);
		expect(decisionsQueryKey(p)).toEqual(['decisions', 's1', 'F', 'T']);
	});

	it('fetchOrders hits the same-origin proxy with from/to encoded', async () => {
		const fetchFn = vi
			.fn()
			.mockResolvedValue(
				new Response('[]', { status: 200, headers: { 'content-type': 'application/json' } })
			);
		await fetchOrders(
			{ strategyId: 'abc def', from: '2026-06-01T00:00:00Z', to: '2026-06-02T00:00:00Z' },
			fetchFn as unknown as typeof fetch
		);
		const url = fetchFn.mock.calls[0][0] as string;
		expect(url).toContain('/api/strategies/abc%20def/orders?');
		expect(url).toContain('from=2026-06-01');
		expect(url).toContain('to=2026-06-02');
	});
});

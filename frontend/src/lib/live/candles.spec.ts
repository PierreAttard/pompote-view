import { describe, expect, it, vi } from 'vitest';
import type { Candle } from '$lib/api/types';
import {
	candlesPath,
	candlesQueryKey,
	fetchCandles,
	mergeCandles,
	type CandlesParams
} from './candles';

function candle(ts: string, close: number): Candle {
	return { ts, o: close, h: close, l: close, c: close, v: 1 };
}

const params: CandlesParams = {
	exchange: 'binance',
	symbol: 'BTCUSDT',
	timeframe: '1h',
	from: '2026-06-01T00:00:00.000Z',
	to: '2026-06-02T00:00:00.000Z'
};

describe('candlesPath', () => {
	it('builds the same-origin proxy URL with every param encoded', () => {
		const url = new URL(candlesPath(params), 'http://localhost');
		expect(url.pathname).toBe('/api/candles');
		expect(url.searchParams.get('exchange')).toBe('binance');
		expect(url.searchParams.get('symbol')).toBe('BTCUSDT');
		expect(url.searchParams.get('timeframe')).toBe('1h');
		expect(url.searchParams.get('from')).toBe(params.from);
		expect(url.searchParams.get('to')).toBe(params.to);
	});
});

describe('candlesQueryKey', () => {
	it('changes when any param changes (so the query refetches)', () => {
		const base = candlesQueryKey(params);
		expect(candlesQueryKey(params)).toEqual(base);
		expect(candlesQueryKey({ ...params, timeframe: '5m' })).not.toEqual(base);
		expect(candlesQueryKey({ ...params, symbol: 'ETHUSDT' })).not.toEqual(base);
	});
});

describe('fetchCandles', () => {
	it('GETs the proxy path through the injected fetch and returns the JSON', async () => {
		const body = [{ ts: '2026-06-01T00:00:00Z', o: 1, h: 2, l: 0.5, c: 1.5, v: 10 }];
		const fetchFn = vi.fn().mockResolvedValue(
			new Response(JSON.stringify(body), {
				status: 200,
				headers: { 'content-type': 'application/json' }
			})
		);

		const out = await fetchCandles(params, fetchFn as unknown as typeof fetch);

		expect(out).toEqual(body);
		expect(fetchFn.mock.calls[0][0] as string).toContain('/api/candles?');
	});

	it('rejects with ApiError on a non-2xx proxy response', async () => {
		const fetchFn = vi.fn().mockResolvedValue(new Response('nope', { status: 400 }));
		await expect(fetchCandles(params, fetchFn as unknown as typeof fetch)).rejects.toMatchObject({
			status: 400
		});
	});
});

describe('mergeCandles', () => {
	it('returns the other side when one is empty', () => {
		const prev = [candle('2026-06-01T00:00:00Z', 1)];
		expect(mergeCandles(prev, [])).toBe(prev);
		expect(mergeCandles([], prev)).toEqual(prev);
	});

	it('appends new candles and updates the boundary (in-progress) bucket', () => {
		const prev = [
			candle('2026-06-01T00:00:00Z', 1),
			candle('2026-06-01T01:00:00Z', 2) // in-progress; fresh re-sends it updated
		];
		const fresh = [
			candle('2026-06-01T01:00:00Z', 9), // same ts, updated close
			candle('2026-06-01T02:00:00Z', 3) // new bar
		];
		expect(mergeCandles(prev, fresh)).toEqual([
			candle('2026-06-01T00:00:00Z', 1),
			candle('2026-06-01T01:00:00Z', 9),
			candle('2026-06-01T02:00:00Z', 3)
		]);
	});

	it('keeps immutable history before the boundary', () => {
		const prev = [candle('2026-06-01T00:00:00Z', 1), candle('2026-06-01T01:00:00Z', 2)];
		const fresh = [candle('2026-06-01T02:00:00Z', 3)];
		expect(mergeCandles(prev, fresh).map((c) => c.ts)).toEqual([
			'2026-06-01T00:00:00Z',
			'2026-06-01T01:00:00Z',
			'2026-06-01T02:00:00Z'
		]);
	});

	it('bounds growth to the cap, keeping the most recent candles', () => {
		const prev = Array.from({ length: 5000 }, (_, i) =>
			candle(new Date(Date.UTC(2026, 0, 1) + i * 60_000).toISOString(), i)
		);
		const fresh = [candle(new Date(Date.UTC(2026, 0, 1) + 5000 * 60_000).toISOString(), 5000)];
		const merged = mergeCandles(prev, fresh);
		expect(merged).toHaveLength(5000);
		expect(merged.at(-1)?.c).toBe(5000); // newest kept
		expect(merged[0].c).toBe(1); // oldest dropped
	});
});

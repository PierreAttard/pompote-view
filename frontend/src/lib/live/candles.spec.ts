import { describe, expect, it, vi } from 'vitest';
import { candlesPath, candlesQueryKey, fetchCandles, type CandlesParams } from './candles';

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

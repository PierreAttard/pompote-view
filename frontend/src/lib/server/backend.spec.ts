import { describe, expect, it, vi } from 'vitest';
import {
	BackendError,
	getBacktestRun,
	getBacktestRunCandles,
	getCandles,
	getStrategyOrders,
	listBacktestRuns,
	type BackendConfig
} from './backend';

const config: BackendConfig = { baseUrl: 'http://backend:3100', apiKey: 'test-key' };

function jsonResponse(body: unknown, status = 200): Response {
	return new Response(JSON.stringify(body), {
		status,
		headers: { 'content-type': 'application/json' }
	});
}

describe('listBacktestRuns', () => {
	it('calls the backtests endpoint with the api key and parses the body', async () => {
		const fetch = vi.fn().mockResolvedValue(jsonResponse([{ id: 'a', symbol: 'BTCUSDT' }]));

		const runs = await listBacktestRuns(fetch as unknown as typeof globalThis.fetch, config);

		expect(runs).toHaveLength(1);
		const [url, init] = fetch.mock.calls[0];
		expect((url as URL).toString()).toBe('http://backend:3100/api/v1/monitoring/backtests');
		expect((init as RequestInit).headers).toEqual({ 'X-API-Key': 'test-key' });
	});

	it('forwards non-empty filters as query params and trims the base url', async () => {
		const fetch = vi.fn().mockResolvedValue(jsonResponse([]));

		await listBacktestRuns(
			fetch as unknown as typeof globalThis.fetch,
			{ ...config, baseUrl: 'http://backend:3100/' },
			{ status: 'completed', symbol: 'BTCUSDT', exchange: undefined }
		);

		const url = fetch.mock.calls[0][0] as URL;
		expect(url.searchParams.get('status')).toBe('completed');
		expect(url.searchParams.get('symbol')).toBe('BTCUSDT');
		expect(url.searchParams.has('exchange')).toBe(false);
		// no double slash from the trailing-slash base url
		expect(url.pathname).toBe('/api/v1/monitoring/backtests');
	});

	it('raises BackendError(500) when the api key is missing', async () => {
		const fetch = vi.fn();
		await expect(
			listBacktestRuns(fetch as unknown as typeof globalThis.fetch, { ...config, apiKey: '' })
		).rejects.toMatchObject({ status: 500 });
		expect(fetch).not.toHaveBeenCalled();
	});

	it('maps a non-2xx response to BackendError with its status', async () => {
		const fetch = vi.fn().mockResolvedValue(jsonResponse({ error: 'unauthorized' }, 401));
		const err = await listBacktestRuns(fetch as unknown as typeof globalThis.fetch, config).catch(
			(e) => e
		);
		expect(err).toBeInstanceOf(BackendError);
		expect(err.status).toBe(401);
	});

	it('maps a network failure to BackendError(502)', async () => {
		const fetch = vi.fn().mockRejectedValue(new TypeError('connection refused'));
		await expect(
			listBacktestRuns(fetch as unknown as typeof globalThis.fetch, config)
		).rejects.toMatchObject({ status: 502 });
	});
});

describe('getCandles', () => {
	it('forwards the locators + window and parses the body', async () => {
		const fetch = vi
			.fn()
			.mockResolvedValue(
				jsonResponse([{ ts: '2026-06-01T00:00:00Z', o: 1, h: 2, l: 0.5, c: 1.5, v: 10 }])
			);

		const candles = await getCandles(fetch as unknown as typeof globalThis.fetch, config, {
			exchange: 'binance',
			symbol: 'BTCUSDT',
			timeframe: '1h',
			from: '2026-06-01T00:00:00Z',
			to: '2026-06-02T00:00:00Z'
		});

		expect(candles).toHaveLength(1);
		const [url, init] = fetch.mock.calls[0];
		expect((url as URL).pathname).toBe('/api/v1/monitoring/candles');
		expect((url as URL).searchParams.get('exchange')).toBe('binance');
		expect((url as URL).searchParams.get('symbol')).toBe('BTCUSDT');
		expect((url as URL).searchParams.get('timeframe')).toBe('1h');
		expect((url as URL).searchParams.get('from')).toBe('2026-06-01T00:00:00Z');
		expect((url as URL).searchParams.get('to')).toBe('2026-06-02T00:00:00Z');
		expect((init as RequestInit).headers).toEqual({ 'X-API-Key': 'test-key' });
	});

	it('omits `to` when not provided', async () => {
		const fetch = vi.fn().mockResolvedValue(jsonResponse([]));
		await getCandles(fetch as unknown as typeof globalThis.fetch, config, {
			exchange: 'binance',
			symbol: 'BTCUSDT',
			timeframe: '1h',
			from: '2026-06-01T00:00:00Z'
		});
		expect((fetch.mock.calls[0][0] as URL).searchParams.has('to')).toBe(false);
	});

	it('maps a 400 (bad timeframe/range) to BackendError with its status', async () => {
		const fetch = vi.fn().mockResolvedValue(jsonResponse({ error: 'invalid_timeframe' }, 400));
		const err = await getCandles(fetch as unknown as typeof globalThis.fetch, config, {
			exchange: 'binance',
			symbol: 'BTCUSDT',
			timeframe: 'bogus',
			from: '2026-06-01T00:00:00Z'
		}).catch((e) => e);
		expect(err).toBeInstanceOf(BackendError);
		expect(err.status).toBe(400);
	});
});

describe('getStrategyOrders', () => {
	it('forwards the strategy id (encoded) + window and parses the body', async () => {
		const fetch = vi
			.fn()
			.mockResolvedValue(jsonResponse([{ order_id: 'o1', side: 'buy', decision_id: 'd1' }]));

		const orders = await getStrategyOrders(
			fetch as unknown as typeof globalThis.fetch,
			config,
			'abc def',
			'2026-06-01T00:00:00Z',
			'2026-06-02T00:00:00Z'
		);

		expect(orders).toHaveLength(1);
		const [url, init] = fetch.mock.calls[0];
		expect((url as URL).pathname).toBe('/api/v1/monitoring/strategies/abc%20def/orders');
		expect((url as URL).searchParams.get('from')).toBe('2026-06-01T00:00:00Z');
		expect((url as URL).searchParams.get('to')).toBe('2026-06-02T00:00:00Z');
		expect((init as RequestInit).headers).toEqual({ 'X-API-Key': 'test-key' });
	});

	it('omits `to` when not provided', async () => {
		const fetch = vi.fn().mockResolvedValue(jsonResponse([]));
		await getStrategyOrders(
			fetch as unknown as typeof globalThis.fetch,
			config,
			's1',
			'2026-06-01T00:00:00Z'
		);
		expect((fetch.mock.calls[0][0] as URL).searchParams.has('to')).toBe(false);
	});
});

describe('getBacktestRun', () => {
	it('returns the detail on 200', async () => {
		const fetch = vi
			.fn()
			.mockResolvedValue(jsonResponse({ run: { id: 'a' }, config_snapshot: {} }));
		const detail = await getBacktestRun(fetch as unknown as typeof globalThis.fetch, config, 'a');
		expect(detail).not.toBeNull();
		const url = fetch.mock.calls[0][0] as URL;
		expect(url.pathname).toBe('/api/v1/monitoring/backtests/a');
	});

	it('returns null on 404', async () => {
		const fetch = vi.fn().mockResolvedValue(jsonResponse({ error: 'not_found' }, 404));
		const detail = await getBacktestRun(
			fetch as unknown as typeof globalThis.fetch,
			config,
			'missing'
		);
		expect(detail).toBeNull();
	});

	it('still raises on other errors', async () => {
		const fetch = vi.fn().mockResolvedValue(jsonResponse({}, 503));
		await expect(
			getBacktestRun(fetch as unknown as typeof globalThis.fetch, config, 'a')
		).rejects.toMatchObject({ status: 503 });
	});
});

describe('getBacktestRunCandles', () => {
	it('passes the timeframe and returns the source hint', async () => {
		const fetch = vi
			.fn()
			.mockResolvedValue(jsonResponse({ source: 'candles_5s', candles: [{}, {}] }));
		const out = await getBacktestRunCandles(
			fetch as unknown as typeof globalThis.fetch,
			config,
			'a',
			'1h'
		);
		expect(out.source).toBe('candles_5s');
		expect(out.candles).toHaveLength(2);
		const url = fetch.mock.calls[0][0] as URL;
		expect(url.pathname).toBe('/api/v1/monitoring/backtests/a/candles');
		expect(url.searchParams.get('timeframe')).toBe('1h');
	});
});

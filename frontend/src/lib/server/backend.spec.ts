import { describe, expect, it, vi } from 'vitest';
import {
	BackendError,
	getBacktestRun,
	getBacktestRunCandles,
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

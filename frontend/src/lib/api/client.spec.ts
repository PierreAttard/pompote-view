import { describe, expect, it, vi } from 'vitest';
import { ApiError, apiGet } from './client';

function jsonResponse(body: unknown, status = 200): Response {
	return new Response(JSON.stringify(body), {
		status,
		headers: { 'content-type': 'application/json' }
	});
}

describe('apiGet', () => {
	it('parses a 2xx JSON body', async () => {
		const fetchFn = vi.fn().mockResolvedValue(jsonResponse([{ id: '1' }]));
		const out = await apiGet<{ id: string }[]>('/api/strategies', fetchFn);
		expect(out).toEqual([{ id: '1' }]);
		expect(fetchFn).toHaveBeenCalledWith('/api/strategies', {
			headers: { accept: 'application/json' }
		});
	});

	it('throws an ApiError flagged unauthorized on 401', async () => {
		const fetchFn = vi.fn().mockResolvedValue(new Response('', { status: 401 }));
		const err = await apiGet('/api/strategies', fetchFn).catch((e) => e);
		expect(err).toBeInstanceOf(ApiError);
		expect((err as ApiError).status).toBe(401);
		expect((err as ApiError).isUnauthorized).toBe(true);
	});

	it('throws an ApiError (not unauthorized) on 5xx', async () => {
		const fetchFn = vi.fn().mockResolvedValue(new Response('', { status: 503 }));
		const err = await apiGet('/api/strategies', fetchFn).catch((e) => e);
		expect(err).toBeInstanceOf(ApiError);
		expect((err as ApiError).status).toBe(503);
		expect((err as ApiError).isUnauthorized).toBe(false);
	});

	it('wraps a network failure as ApiError(status 0)', async () => {
		const fetchFn = vi.fn().mockRejectedValue(new TypeError('offline'));
		const err = await apiGet('/api/strategies', fetchFn).catch((e) => e);
		expect(err).toBeInstanceOf(ApiError);
		expect((err as ApiError).status).toBe(0);
	});
});

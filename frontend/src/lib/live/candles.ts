/**
 * Browser-side helpers for fetching live candles through the same-origin proxy.
 *
 * The component never calls the Rust backend directly (the `X-API-Key` stays
 * server-side): it hits `/api/candles`, which `+server.ts` forwards with the
 * key injected. These helpers build that request and the TanStack Query key, so
 * the query is unit-testable without a component.
 */
import { apiGet } from '$lib/api/client';
import type { Candle } from '$lib/api/types';

/** Locators + `[from, to)` window identifying one candles query. */
export interface CandlesParams {
	exchange: string;
	symbol: string;
	timeframe: string;
	from: string;
	to: string;
}

/** Same-origin `/api/candles?…` path for the given params. */
export function candlesPath(params: CandlesParams): string {
	const qs = new URLSearchParams({
		exchange: params.exchange,
		symbol: params.symbol,
		timeframe: params.timeframe,
		from: params.from,
		to: params.to
	});
	return `/api/candles?${qs.toString()}`;
}

/**
 * Stable TanStack Query key for a candles request. Changing any selector (and
 * thus a param) yields a new key, so the query refetches and the chart updates.
 */
export function candlesQueryKey(params: CandlesParams) {
	return ['candles', params.exchange, params.symbol, params.timeframe, params.from, params.to];
}

/** Fetches the candles for `params` via the same-origin proxy. */
export function fetchCandles(
	params: CandlesParams,
	fetchFn: typeof fetch = fetch
): Promise<Candle[]> {
	return apiGet<Candle[]>(candlesPath(params), fetchFn);
}

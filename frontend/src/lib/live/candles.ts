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
import { MAX_CANDLE_POINTS } from './depth';

/** Live polling cadence (#26): refetch the tail every 10s. */
export const POLL_INTERVAL_MS = 10_000;

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

/**
 * Merges a freshly-polled tail into the accumulated candles (#26).
 *
 * The poll refetches `[lastTs, now]`, so `fresh[0]` is the boundary bucket the
 * previous result already held (still in-progress, possibly updated): drop any
 * accumulated candle at/after that timestamp, then append the fresh tail. History
 * before the boundary is immutable and kept as-is. The result is bounded to the
 * last `MAX_CANDLE_POINTS` so a long live session can't grow without limit.
 */
export function mergeCandles(prev: Candle[], fresh: Candle[]): Candle[] {
	if (fresh.length === 0) return prev;
	if (prev.length === 0) return fresh.slice(-MAX_CANDLE_POINTS);
	const cutoff = Date.parse(fresh[0].ts);
	const kept = Number.isNaN(cutoff) ? prev : prev.filter((c) => Date.parse(c.ts) < cutoff);
	const merged = [...kept, ...fresh];
	return merged.length > MAX_CANDLE_POINTS ? merged.slice(-MAX_CANDLE_POINTS) : merged;
}

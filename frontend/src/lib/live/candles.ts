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
	if (fresh.length === 0) return prev; // ref-stable: a poll with nothing new is a no-op
	// Index by timestamp (fresh wins, so the in-progress boundary bucket updates),
	// then sort ascending and bound to the cap. Order-independent: the accumulated
	// result stays sorted and de-duplicated regardless of the input ordering.
	const byTs: Record<string, Candle> = {};
	for (const c of prev) byTs[c.ts] = c;
	for (const c of fresh) byTs[c.ts] = c;
	const merged = Object.values(byTs).sort((a, b) => Date.parse(a.ts) - Date.parse(b.ts));
	return merged.length > MAX_CANDLE_POINTS ? merged.slice(-MAX_CANDLE_POINTS) : merged;
}

/**
 * Lower-bounds the incremental poll `from` (RFC3339) so a long-hidden tab's
 * catch-up can't request a window beyond the depth cap (which the backend would
 * reject with a 400, freezing the chart). Returns the later of `lastTs` and
 * `now - MAX_CANDLE_POINTS * timeframe`; on the first poll `lastTs` is the window
 * start, which is already cap-safe (the depth guard gates rendering).
 */
export function boundedPollFrom(lastTs: string, nowMs: number, timeframeSeconds: number): string {
	const lastMs = Date.parse(lastTs);
	if (Number.isNaN(lastMs) || timeframeSeconds <= 0) return lastTs;
	const minMs = nowMs - MAX_CANDLE_POINTS * timeframeSeconds * 1000;
	return new Date(Math.max(lastMs, minMs)).toISOString();
}

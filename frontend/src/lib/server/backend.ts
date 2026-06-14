/**
 * Server-only client for the read-only viz backend.
 *
 * Lives under `$lib/server/` so SvelteKit guarantees it is never bundled into
 * the browser: the `X-API-Key` is a shared secret and must stay server-side.
 * Page `load` functions read the config from private env and call these
 * helpers; the browser only ever sees the already-fetched data.
 *
 * Types are sourced from the generated OpenAPI client (`$lib/api/types.gen`),
 * so a backend contract change surfaces here at compile time. A fuller typed
 * client (all endpoints, retries) is issue #16; this module currently covers
 * the backtest run listing needed by #51.
 */
import type {
	BacktestCandles,
	BacktestOrder,
	BacktestRunDetail,
	BacktestRunSummary,
	Decision
} from '$lib/api/types';

export type { BacktestCandles, BacktestOrder, BacktestRunDetail, BacktestRunSummary, Decision };

/** Connection config, built from private env by the caller. */
export interface BackendConfig {
	/** Base URL of the viz backend, e.g. `http://127.0.0.1:3100`. */
	baseUrl: string;
	/** Value sent in the `X-API-Key` header. */
	apiKey: string;
}

/** Optional AND-combined filters for the run listing. */
export interface BacktestRunFilters {
	status?: string;
	strategy_kind?: string;
	exchange?: string;
	symbol?: string;
}

/** Error raised when the backend call fails (mapped to an HTTP status). */
export class BackendError extends Error {
	constructor(
		readonly status: number,
		message: string,
		options?: ErrorOptions
	) {
		super(message, options);
		this.name = 'BackendError';
	}
}

/**
 * Fetches the backtest runs (the selector list).
 *
 * `fetch` is injected (SvelteKit's `event.fetch` in production, a stub in
 * tests) so this function stays pure and unit-testable without a real server.
 */
export async function listBacktestRuns(
	fetch: typeof globalThis.fetch,
	config: BackendConfig,
	filters: BacktestRunFilters = {}
): Promise<BacktestRunSummary[]> {
	const url = new URL(`${trimBase(config)}/api/v1/monitoring/backtests`);
	for (const [key, value] of Object.entries(filters)) {
		if (value) url.searchParams.set(key, value);
	}

	return backendGet<BacktestRunSummary[]>(fetch, config, url);
}

/**
 * Fetches a single run with its `config_snapshot`, or `null` when the backend
 * returns `404` (so the page can render a clean "not found").
 */
export async function getBacktestRun(
	fetch: typeof globalThis.fetch,
	config: BackendConfig,
	runId: string
): Promise<BacktestRunDetail | null> {
	const url = new URL(
		`${trimBase(config)}/api/v1/monitoring/backtests/${encodeURIComponent(runId)}`
	);
	return backendGet<BacktestRunDetail | null>(fetch, config, url, { notFoundAsNull: true });
}

/**
 * Fetches the run's aggregated candles plus the `source` hint
 * (`candles_5s` | `none`) that drives the degraded, timeline-only view.
 */
export async function getBacktestRunCandles(
	fetch: typeof globalThis.fetch,
	config: BackendConfig,
	runId: string,
	timeframe: string
): Promise<BacktestCandles> {
	const url = new URL(
		`${trimBase(config)}/api/v1/monitoring/backtests/${encodeURIComponent(runId)}/candles`
	);
	url.searchParams.set('timeframe', timeframe);
	return backendGet<BacktestCandles>(fetch, config, url);
}

/**
 * Fetches the run's orders (buy/sell), ordered by ascending `market_ts`. These
 * back the chart's decision markers.
 */
export async function getBacktestRunOrders(
	fetch: typeof globalThis.fetch,
	config: BackendConfig,
	runId: string
): Promise<BacktestOrder[]> {
	const url = new URL(
		`${trimBase(config)}/api/v1/monitoring/backtests/${encodeURIComponent(runId)}/orders`
	);
	return backendGet<BacktestOrder[]>(fetch, config, url);
}

/**
 * Fetches the run's decisions (with their opaque market-context `snapshot`),
 * ordered by ascending `market_ts`. These back the indicator overlays.
 */
export async function getBacktestRunDecisions(
	fetch: typeof globalThis.fetch,
	config: BackendConfig,
	runId: string
): Promise<Decision[]> {
	const url = new URL(
		`${trimBase(config)}/api/v1/monitoring/backtests/${encodeURIComponent(runId)}/decisions`
	);
	return backendGet<Decision[]>(fetch, config, url);
}

function trimBase(config: BackendConfig): string {
	return config.baseUrl.replace(/\/+$/, '');
}

/** Shared GET with api-key header, error mapping and optional `404 -> null`. */
async function backendGet<T>(
	fetch: typeof globalThis.fetch,
	config: BackendConfig,
	url: URL,
	opts: { notFoundAsNull?: boolean } = {}
): Promise<T> {
	if (!config.apiKey) {
		throw new BackendError(500, 'VIZ_API_KEY is not configured on the viz frontend server');
	}

	let res: Response;
	try {
		res = await fetch(url, { headers: { 'X-API-Key': config.apiKey } });
	} catch (cause) {
		throw new BackendError(502, `viz backend unreachable at ${config.baseUrl}`, { cause });
	}
	if (opts.notFoundAsNull && res.status === 404) {
		return null as T;
	}
	if (!res.ok) {
		throw new BackendError(res.status, `viz backend returned ${res.status} for ${url.pathname}`);
	}
	return (await res.json()) as T;
}

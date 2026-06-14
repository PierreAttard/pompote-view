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
	BacktestMetrics,
	BacktestOrder,
	BacktestRunDetail,
	BacktestRunSummary,
	Candle,
	Decision,
	LiveDecision,
	LiveFill,
	Order,
	Strategy
} from '$lib/api/types';

export type {
	BacktestCandles,
	BacktestMetrics,
	BacktestOrder,
	BacktestRunDetail,
	BacktestRunSummary,
	Candle,
	Decision,
	LiveDecision,
	LiveFill,
	Order,
	Strategy
};

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

/**
 * Fetches the run's recomputed performance metrics (PnL/fees from
 * `fills_backtest`). Best-effort at the call site: the detail page renders
 * fine without them.
 */
export async function getBacktestRunMetrics(
	fetch: typeof globalThis.fetch,
	config: BackendConfig,
	runId: string
): Promise<BacktestMetrics> {
	const url = new URL(
		`${trimBase(config)}/api/v1/monitoring/backtests/${encodeURIComponent(runId)}/metrics`
	);
	return backendGet<BacktestMetrics>(fetch, config, url);
}

/** Required `[from, to)` window + locators for the live candles query. */
export interface CandlesQuery {
	exchange: string;
	symbol: string;
	timeframe: string;
	/** Inclusive lower bound on `open_time` (RFC3339). */
	from: string;
	/** Exclusive upper bound (RFC3339). Omit to let the backend default to now. */
	to?: string;
}

/**
 * Fetches the live OHLCV candles for `[from, to)`, ordered by ascending `ts`.
 *
 * The backend caps the response at 5000 buckets and rejects unknown timeframes
 * (HTTP 400, mapped to a `BackendError`); callers surface that to the user.
 */
export async function getCandles(
	fetch: typeof globalThis.fetch,
	config: BackendConfig,
	query: CandlesQuery
): Promise<Candle[]> {
	const url = new URL(`${trimBase(config)}/api/v1/monitoring/candles`);
	url.searchParams.set('exchange', query.exchange);
	url.searchParams.set('symbol', query.symbol);
	url.searchParams.set('timeframe', query.timeframe);
	url.searchParams.set('from', query.from);
	if (query.to) url.searchParams.set('to', query.to);
	return backendGet<Candle[]>(fetch, config, url);
}

/** Fetches every strategy (the live selector), with paper/live mode flags. */
export async function listStrategies(
	fetch: typeof globalThis.fetch,
	config: BackendConfig
): Promise<Strategy[]> {
	const url = new URL(`${trimBase(config)}/api/v1/monitoring/strategies`);
	return backendGet<Strategy[]>(fetch, config, url);
}

/** Fetches the allowed aggregation timeframes (finest to coarsest). */
export async function getTimeframes(
	fetch: typeof globalThis.fetch,
	config: BackendConfig
): Promise<string[]> {
	const url = new URL(`${trimBase(config)}/api/v1/monitoring/timeframes`);
	return backendGet<string[]>(fetch, config, url);
}

/**
 * Fetches a strategy's live fills (execution markers) on a `[from, to)` window
 * over `executed_at`.
 */
export async function getStrategyFills(
	fetch: typeof globalThis.fetch,
	config: BackendConfig,
	strategyId: string,
	from: string,
	to?: string
): Promise<LiveFill[]> {
	const url = new URL(
		`${trimBase(config)}/api/v1/monitoring/strategies/${encodeURIComponent(strategyId)}/fills`
	);
	url.searchParams.set('from', from);
	if (to) url.searchParams.set('to', to);
	return backendGet<LiveFill[]>(fetch, config, url);
}

/**
 * Fetches a strategy's live decisions (with market context) on a `[from, to)`
 * window over `created_at`.
 */
export async function getStrategyDecisions(
	fetch: typeof globalThis.fetch,
	config: BackendConfig,
	strategyId: string,
	from: string,
	to?: string
): Promise<LiveDecision[]> {
	const url = new URL(
		`${trimBase(config)}/api/v1/monitoring/strategies/${encodeURIComponent(strategyId)}/decisions`
	);
	url.searchParams.set('from', from);
	if (to) url.searchParams.set('to', to);
	return backendGet<LiveDecision[]>(fetch, config, url);
}

/**
 * Fetches a strategy's live orders (buy/sell, with their `decision_id`) on a
 * `[from, to)` window over `created_at`. These back the live chart's decision
 * markers; the `decision_id` joins each marker to its rationale + snapshot.
 */
export async function getStrategyOrders(
	fetch: typeof globalThis.fetch,
	config: BackendConfig,
	strategyId: string,
	from: string,
	to?: string
): Promise<Order[]> {
	const url = new URL(
		`${trimBase(config)}/api/v1/monitoring/strategies/${encodeURIComponent(strategyId)}/orders`
	);
	url.searchParams.set('from', from);
	if (to) url.searchParams.set('to', to);
	return backendGet<Order[]>(fetch, config, url);
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

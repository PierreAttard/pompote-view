/**
 * Live decision annotations for the chart: orders become buy/sell markers, and
 * each marker joins back to its decision (rationale + market-context snapshot)
 * via `order.decision_id`.
 *
 * The fetch helpers go through the same-origin proxies (`X-API-Key` stays
 * server-side); the mappers are pure so the join/marker logic is unit-testable
 * without a component.
 */
import { apiGet } from '$lib/api/client';
import type { LiveDecision, Order } from '$lib/api/types';
import type { ChartMarker } from '$lib/chart/markers';

/** `[from, to)` window for the annotation queries (mirrors the candle window). */
export interface AnnotationParams {
	strategyId: string;
	from: string;
	to: string;
}

/** Map live orders to buy/sell chart markers (`B`/`S`/`?` labels). */
export function ordersToMarkers(orders: Order[]): ChartMarker[] {
	return orders.map((o) => {
		// Defensive: API rows can contradict the `string` type at runtime.
		const side = String(o.side ?? '').toLowerCase();
		return {
			ts: o.created_at,
			side: o.side,
			text: side === 'buy' ? 'B' : side === 'sell' ? 'S' : '?'
		};
	});
}

/** Index decisions by their id for an O(1) `order -> decision` join. */
export function indexDecisionsById(decisions: LiveDecision[]): Map<string, LiveDecision> {
	const byId = new Map<string, LiveDecision>();
	for (const d of decisions) byId.set(d.decision_id, d);
	return byId;
}

/**
 * Start (epoch seconds) of the candle bucket an RFC3339 timestamp falls into,
 * for a timeframe of `timeframeSeconds`. Markers and the hover lookup are keyed
 * on this so a crosshair landing on a bar resolves to the orders in that bucket.
 * Returns `null` for an unparseable timestamp or non-positive width.
 */
export function bucketStartSeconds(ts: string, timeframeSeconds: number): number | null {
	if (timeframeSeconds <= 0) return null;
	const ms = Date.parse(ts);
	if (Number.isNaN(ms)) return null;
	const sec = Math.floor(ms / 1000);
	return Math.floor(sec / timeframeSeconds) * timeframeSeconds;
}

/**
 * Builds the hover lookup: candle-bucket start (epoch seconds) → the decisions
 * whose orders fall in that bucket. Orders with no matching decision are
 * skipped; a bucket can hold several decisions (de-duplicated by id, preserving
 * first-seen order).
 */
export function buildDecisionLookup(
	orders: Order[],
	decisions: LiveDecision[],
	timeframeSeconds: number
): Map<number, LiveDecision[]> {
	const byId = indexDecisionsById(decisions);
	const lookup = new Map<number, LiveDecision[]>();
	for (const order of orders) {
		const decision = byId.get(order.decision_id);
		if (!decision) continue;
		const bucket = bucketStartSeconds(order.created_at, timeframeSeconds);
		if (bucket === null) continue;
		const list = lookup.get(bucket) ?? [];
		if (!list.some((d) => d.decision_id === decision.decision_id)) {
			list.push(decision);
			lookup.set(bucket, list);
		}
	}
	return lookup;
}

function strategyPath(
	strategyId: string,
	resource: 'orders' | 'decisions',
	from: string,
	to: string
) {
	const qs = new URLSearchParams({ from, to });
	return `/api/strategies/${encodeURIComponent(strategyId)}/${resource}?${qs.toString()}`;
}

/** TanStack Query key for a strategy's live orders on a window. */
export function ordersQueryKey(params: AnnotationParams) {
	return ['orders', params.strategyId, params.from, params.to];
}

/** TanStack Query key for a strategy's live decisions on a window. */
export function decisionsQueryKey(params: AnnotationParams) {
	return ['decisions', params.strategyId, params.from, params.to];
}

/** Fetches the strategy's live orders via the same-origin proxy. */
export function fetchOrders(
	params: AnnotationParams,
	fetchFn: typeof fetch = fetch
): Promise<Order[]> {
	return apiGet<Order[]>(
		strategyPath(params.strategyId, 'orders', params.from, params.to),
		fetchFn
	);
}

/** Fetches the strategy's live decisions via the same-origin proxy. */
export function fetchDecisions(
	params: AnnotationParams,
	fetchFn: typeof fetch = fetch
): Promise<LiveDecision[]> {
	return apiGet<LiveDecision[]>(
		strategyPath(params.strategyId, 'decisions', params.from, params.to),
		fetchFn
	);
}

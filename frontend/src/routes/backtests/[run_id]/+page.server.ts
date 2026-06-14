import { error } from '@sveltejs/kit';
import {
	BackendError,
	getBacktestRun,
	getBacktestRunCandles,
	getBacktestRunDecisions,
	getBacktestRunMetrics,
	getBacktestRunOrders,
	type BacktestCandles,
	type BacktestMetrics
} from '$lib/server/backend';
import { backendConfig } from '$lib/server/config';
import { decisionsToOverlays, ordersToMarkers } from '$lib/backtest/series';
import type { PageServerLoad } from './$types';

/** Default aggregation for the detail view's candle background. */
const DEFAULT_TIMEFRAME = '1h';

/**
 * Loads a single run plus a candle availability probe. The candles are
 * best-effort: when the window is not covered by `candles_5s` (or the probe
 * fails), the view degrades to timeline-only rather than erroring.
 */
export const load: PageServerLoad = async ({ fetch, params }) => {
	const config = backendConfig();

	let detail;
	try {
		detail = await getBacktestRun(fetch, config, params.run_id);
	} catch (e) {
		throw error(e instanceof BackendError ? e.status : 502, 'Impossible de charger le run');
	}
	if (!detail) {
		throw error(404, 'Run de backtest introuvable');
	}

	// Fetch the candle background plus the `source` hint. Swallow failures: the
	// run detail is what matters; the chart background is optional and absent
	// candles simply degrade to the timeline-only view.
	let source = 'none';
	let candles: BacktestCandles['candles'] = [];
	try {
		const probe = await getBacktestRunCandles(fetch, config, params.run_id, DEFAULT_TIMEFRAME);
		source = probe.source;
		candles = probe.candles;
	} catch {
		// Degraded view: candles unavailable -> keep source='none', candles=[].
	}

	// Orders -> buy/sell markers; decision snapshots -> indicator overlays. Both
	// are best-effort annotations: a failure must not break the detail page.
	let markers: ReturnType<typeof ordersToMarkers> = [];
	let overlays: ReturnType<typeof decisionsToOverlays> = [];
	try {
		markers = ordersToMarkers(await getBacktestRunOrders(fetch, config, params.run_id));
	} catch {
		// Markers unavailable -> chart renders without annotations.
	}
	try {
		overlays = decisionsToOverlays(await getBacktestRunDecisions(fetch, config, params.run_id));
	} catch {
		// Overlays unavailable -> chart renders without indicator lines.
	}

	// Recomputed PnL/fees metrics (V1+1). Best-effort: a failure must not break
	// the detail page, so `null` simply hides the metrics panel.
	let metrics: BacktestMetrics | null = null;
	try {
		metrics = await getBacktestRunMetrics(fetch, config, params.run_id);
	} catch {
		// Metrics unavailable -> panel hidden.
	}

	return {
		detail,
		source,
		candles,
		candleCount: candles.length,
		markers,
		overlays,
		metrics,
		timeframe: DEFAULT_TIMEFRAME
	};
};

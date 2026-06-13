import { error } from '@sveltejs/kit';
import { env } from '$env/dynamic/private';
import {
	BackendError,
	getBacktestRun,
	getBacktestRunCandles,
	type BackendConfig
} from '$lib/server/backend';
import type { PageServerLoad } from './$types';

const DEFAULT_BACKEND_URL = 'http://127.0.0.1:3100';
/** Default aggregation for the detail view's candle background. */
const DEFAULT_TIMEFRAME = '1h';

/**
 * Loads a single run plus a candle availability probe. The candles are
 * best-effort: when the window is not covered by `candles_5s` (or the probe
 * fails), the view degrades to timeline-only rather than erroring.
 */
export const load: PageServerLoad = async ({ fetch, params }) => {
	const config: BackendConfig = {
		baseUrl: env.BACKEND_URL || DEFAULT_BACKEND_URL,
		apiKey: env.VIZ_API_KEY ?? ''
	};

	let detail;
	try {
		detail = await getBacktestRun(fetch, config, params.run_id);
	} catch (e) {
		throw error(e instanceof BackendError ? e.status : 502, 'Impossible de charger le run');
	}
	if (!detail) {
		throw error(404, 'Run de backtest introuvable');
	}

	// Probe candle coverage for the degraded-view decision. Swallow failures:
	// the run detail is what matters; the chart background is optional.
	let source = 'none';
	let candleCount = 0;
	try {
		const candles = await getBacktestRunCandles(fetch, config, params.run_id, DEFAULT_TIMEFRAME);
		source = candles.source;
		candleCount = candles.candles.length;
	} catch {
		// Degraded view: candles unavailable -> keep source='none', candleCount=0.
	}

	return { detail, source, candleCount, timeframe: DEFAULT_TIMEFRAME };
};

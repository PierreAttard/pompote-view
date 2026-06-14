/**
 * Same-origin proxy for the live OHLC candles.
 *
 * The browser hits `/api/candles?exchange=…&symbol=…&timeframe=…&from=…[&to=…]`
 * (no secret); this server route injects the `X-API-Key` and forwards to the viz
 * backend. Keeping the key server-side is the whole point of proxying rather
 * than calling the backend from the browser.
 *
 * Required params are validated here so a malformed client request fails fast
 * with a clear 400 instead of bubbling an opaque backend error.
 */
import { error, json } from '@sveltejs/kit';
import { BackendError, getCandles } from '$lib/server/backend';
import { backendConfig } from '$lib/server/config';
import type { RequestHandler } from './$types';

/** Query params the candles endpoint cannot run without. */
const REQUIRED = ['exchange', 'symbol', 'timeframe', 'from'] as const;

export const GET: RequestHandler = async ({ fetch, url }) => {
	const missing = REQUIRED.filter((key) => !url.searchParams.get(key));
	if (missing.length > 0) {
		throw error(400, `Paramètres manquants : ${missing.join(', ')}`);
	}

	try {
		const candles = await getCandles(fetch, backendConfig(), {
			exchange: url.searchParams.get('exchange')!,
			symbol: url.searchParams.get('symbol')!,
			timeframe: url.searchParams.get('timeframe')!,
			from: url.searchParams.get('from')!,
			to: url.searchParams.get('to') ?? undefined
		});
		return json(candles);
	} catch (e) {
		// Server-side log for debugging; nothing sensitive reaches the browser.
		console.error('[/api/candles] backend call failed', e);
		throw error(e instanceof BackendError ? e.status : 502, 'Impossible de charger les bougies');
	}
};

/**
 * Same-origin proxy for a strategy's live orders (the chart's buy/sell markers).
 *
 * The browser hits `/api/strategies/:id/orders?from=…[&to=…]` (no secret); this
 * route injects the `X-API-Key` and forwards to the viz backend. Each order
 * carries a `decision_id` so the UI can join it to the decision rationale.
 */
import { error, json } from '@sveltejs/kit';
import { BackendError, getStrategyOrders } from '$lib/server/backend';
import { backendConfig } from '$lib/server/config';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ fetch, params, url }) => {
	const from = url.searchParams.get('from');
	if (!from) throw error(400, 'Paramètre manquant : from');

	try {
		const orders = await getStrategyOrders(
			fetch,
			backendConfig(),
			params.id,
			from,
			url.searchParams.get('to') ?? undefined
		);
		return json(orders);
	} catch (e) {
		console.error('[/api/strategies/:id/orders] backend call failed', e);
		throw error(e instanceof BackendError ? e.status : 502, 'Impossible de charger les ordres');
	}
};

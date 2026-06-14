/**
 * Same-origin proxy for a strategy's live fills (order executions).
 *
 * The browser hits `/api/strategies/:id/fills?from=…[&to=…]` (no secret); this
 * route injects the `X-API-Key` and forwards to the viz backend. Each fill joins
 * back to its order via `order_id`, so the decision detail panel can list the
 * executions of a decision's orders (#22).
 */
import { error, json } from '@sveltejs/kit';
import { BackendError, getStrategyFills } from '$lib/server/backend';
import { backendConfig } from '$lib/server/config';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ fetch, params, url }) => {
	const from = url.searchParams.get('from');
	if (!from) throw error(400, 'Paramètre manquant : from');

	try {
		const fills = await getStrategyFills(
			fetch,
			backendConfig(),
			params.id,
			from,
			url.searchParams.get('to') ?? undefined
		);
		return json(fills);
	} catch (e) {
		console.error('[/api/strategies/:id/fills] backend call failed', e);
		throw error(e instanceof BackendError ? e.status : 502, 'Impossible de charger les exécutions');
	}
};

/**
 * Same-origin proxy for a strategy's live decisions (rationale + opaque market
 * context snapshot).
 *
 * The browser hits `/api/strategies/:id/decisions?from=…[&to=…]` (no secret);
 * this route injects the `X-API-Key` and forwards to the viz backend. The
 * `decision_id` joins each decision to the orders (markers) it emitted.
 */
import { error, json } from '@sveltejs/kit';
import { BackendError, getStrategyDecisions } from '$lib/server/backend';
import { backendConfig } from '$lib/server/config';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ fetch, params, url }) => {
	const from = url.searchParams.get('from');
	if (!from) throw error(400, 'Paramètre manquant : from');

	try {
		const decisions = await getStrategyDecisions(
			fetch,
			backendConfig(),
			params.id,
			from,
			url.searchParams.get('to') ?? undefined
		);
		return json(decisions);
	} catch (e) {
		console.error('[/api/strategies/:id/decisions] backend call failed', e);
		throw error(e instanceof BackendError ? e.status : 502, 'Impossible de charger les décisions');
	}
};

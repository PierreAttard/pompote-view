/**
 * Same-origin proxy for the strategy selector.
 *
 * The browser hits `/api/strategies` (no secret); this server route injects the
 * `X-API-Key` and forwards to the viz backend. Keeping the key server-side is
 * the whole point of proxying rather than calling the backend from the browser.
 */
import { error, json } from '@sveltejs/kit';
import { BackendError, listStrategies } from '$lib/server/backend';
import { backendConfig } from '$lib/server/config';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ fetch }) => {
	try {
		return json(await listStrategies(fetch, backendConfig()));
	} catch (e) {
		throw error(e instanceof BackendError ? e.status : 502, 'Impossible de charger les stratégies');
	}
};

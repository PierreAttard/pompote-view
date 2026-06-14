import { error } from '@sveltejs/kit';
import { BackendError, listBacktestRuns, type BacktestRunFilters } from '$lib/server/backend';
import { backendConfig } from '$lib/server/config';
import type { PageServerLoad } from './$types';

/**
 * Loads the backtest runs server-side so the `X-API-Key` never reaches the
 * browser. Filters come from the URL query string (the selector form submits a
 * plain GET), keeping them shareable/bookmarkable.
 */
export const load: PageServerLoad = async ({ fetch, url }) => {
	const config = backendConfig();

	const filters: BacktestRunFilters = {
		status: url.searchParams.get('status') ?? undefined,
		strategy_kind: url.searchParams.get('strategy_kind') ?? undefined,
		exchange: url.searchParams.get('exchange') ?? undefined,
		symbol: url.searchParams.get('symbol') ?? undefined
	};

	try {
		const runs = await listBacktestRuns(fetch, config, filters);
		return { runs, filters };
	} catch (e) {
		const status = e instanceof BackendError ? e.status : 502;
		throw error(status, 'Impossible de charger les runs de backtest');
	}
};

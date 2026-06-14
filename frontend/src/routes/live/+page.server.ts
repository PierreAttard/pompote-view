import { error } from '@sveltejs/kit';
import { BackendError, getTimeframes, listStrategies } from '$lib/server/backend';
import { backendConfig } from '$lib/server/config';
import type { PageServerLoad } from './$types';

/**
 * Loads the data backing the live selectors (strategies + timeframes)
 * server-side so the `X-API-Key` stays off the browser. The current selection
 * lives in the URL query string (shareable/bookmarkable); the page reads it
 * from `$page.url` and echoes it back into the controls.
 */
export const load: PageServerLoad = async ({ fetch }) => {
	const config = backendConfig();
	try {
		const [strategies, timeframes] = await Promise.all([
			listStrategies(fetch, config),
			getTimeframes(fetch, config)
		]);
		return { strategies, timeframes };
	} catch (e) {
		throw error(e instanceof BackendError ? e.status : 502, 'Impossible de charger les sélecteurs');
	}
};

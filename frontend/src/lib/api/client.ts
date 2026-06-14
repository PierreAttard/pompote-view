/**
 * Browser-side HTTP client for the viz frontend.
 *
 * **Security:** the `X-API-Key` is a shared secret and stays server-side. The
 * browser therefore never calls the Rust backend directly — it calls
 * same-origin SvelteKit endpoints (`/api/...` → `+server.ts`) that proxy to the
 * backend with the key injected server-side (see `$lib/server/backend.ts`).
 * This wrapper consequently sends **no** auth header; it only does uniform
 * error handling + typed JSON parsing for those same-origin calls.
 *
 * Pair it with `@tanstack/svelte-query` (provider mounted at the root layout)
 * for caching, dedup and the 10s polling used by live monitoring (Lot 7).
 */

/** Error thrown for a non-2xx (or unreachable) same-origin API response. */
export class ApiError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
		this.name = 'ApiError';
	}

	/** `true` when the proxy reported a missing/invalid API key (HTTP 401). */
	get isUnauthorized(): boolean {
		return this.status === 401;
	}
}

/**
 * Typed GET against a same-origin SvelteKit API route.
 *
 * `fetchFn` is injectable (SvelteKit's `event.fetch` in `load`, the global
 * `fetch` in the browser, a stub in tests). Throws [`ApiError`] on network
 * failure or any non-2xx status; logs to the console in dev.
 */
export async function apiGet<T>(path: string, fetchFn: typeof fetch = fetch): Promise<T> {
	let res: Response;
	try {
		res = await fetchFn(path, { headers: { accept: 'application/json' } });
	} catch (cause) {
		const message = `network error calling ${path}`;
		if (import.meta.env.DEV) console.error('[api]', message, cause);
		throw new ApiError(0, message);
	}
	if (!res.ok) {
		const message = `request to ${path} failed (${res.status})`;
		if (import.meta.env.DEV) console.error('[api]', message);
		throw new ApiError(res.status, message);
	}
	return (await res.json()) as T;
}

/**
 * Server-only helper building the viz backend connection config from private
 * env. Centralised so every `load` function and `+server.ts` proxy route reads
 * the same `BACKEND_URL` / `VIZ_API_KEY` (the API key never leaves the server).
 */
import { env } from '$env/dynamic/private';
import type { BackendConfig } from './backend';

/** Default backend URL when `BACKEND_URL` is unset (local dev). */
const DEFAULT_BACKEND_URL = 'http://127.0.0.1:3100';

/** Builds the backend config from private env. */
export function backendConfig(): BackendConfig {
	return {
		baseUrl: env.BACKEND_URL || DEFAULT_BACKEND_URL,
		apiKey: env.VIZ_API_KEY ?? ''
	};
}

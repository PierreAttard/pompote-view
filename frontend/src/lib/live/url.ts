/**
 * Pure URL-query helper for the live selectors. Kept Svelte-free so the
 * persistence logic is unit-testable without a router.
 */

/**
 * Builds the next query string after applying `changes` to `current`.
 *
 * Each key in `changes` is replaced (any existing occurrences removed first); a
 * `null` or empty value clears the key. Other params are preserved. Returns the
 * encoded query string (without the leading `?`).
 */
export function nextQuery(
	current: URLSearchParams,
	changes: Record<string, string | null>
): string {
	const replaced = new Set(Object.keys(changes));
	const entries = [...current].filter(([k]) => !replaced.has(k));
	for (const [k, v] of Object.entries(changes)) {
		if (v !== null && v !== '') entries.push([k, v]);
	}
	return new URLSearchParams(entries).toString();
}

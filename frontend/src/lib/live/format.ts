/**
 * Small, pure display formatters for the live views. DOM-free so they unit-test
 * in the node project.
 */

/**
 * Formats an RFC3339 timestamp as `JJ/MM/AAAA HH:mm:ss` in **UTC** (the chart
 * and the backend both speak UTC, so we avoid a misleading local-time shift).
 * Returns the raw string unchanged when it can't be parsed.
 */
export function formatTimestamp(iso: string): string {
	const ms = Date.parse(iso);
	if (Number.isNaN(ms)) return iso;
	return new Intl.DateTimeFormat('fr-FR', {
		year: 'numeric',
		month: '2-digit',
		day: '2-digit',
		hour: '2-digit',
		minute: '2-digit',
		second: '2-digit',
		hour12: false,
		timeZone: 'UTC'
	}).format(ms);
}

/**
 * Formats a numeric cell (price/quantity/fee), trimming floating-point noise and
 * trailing zeros (crypto values can carry up to 8 decimals). `null`/`undefined`/
 * non-finite render as an em dash.
 */
export function formatNumber(value: number | null | undefined): string {
	if (value === null || value === undefined || !Number.isFinite(value)) return '—';
	if (Number.isInteger(value)) return String(value);
	return String(Number(value.toFixed(8)));
}

/**
 * Formats an elapsed duration (ms) as a short French "il y a …" label for the
 * freshness indicator (#27): seconds under a minute, then minutes, then hours.
 * Negative/sub-second ages read "à l'instant".
 */
export function formatAge(ms: number): string {
	const s = Math.floor(ms / 1000);
	if (s < 1) return "à l'instant";
	if (s < 60) return `il y a ${s} s`;
	const m = Math.floor(s / 60);
	if (m < 60) return `il y a ${m} min`;
	return `il y a ${Math.floor(m / 60)} h`;
}

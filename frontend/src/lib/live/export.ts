/**
 * Export helpers for the live chart (#33): the current window's candles and
 * decisions as CSV, and the chart as a PNG.
 *
 * The CSV builders are pure (DOM-free) so they unit-test in the node project;
 * the download helpers touch the DOM and only run in the browser (they reference
 * `document`/`Blob`/`URL` lazily, inside the functions, so importing this module
 * in node stays safe).
 */
import type { Candle, LiveDecision } from '$lib/api/types';

/** Quotes a CSV field when it contains a delimiter, quote or newline (RFC 4180). */
export function escapeCsvField(value: string): string {
	return /[",\r\n]/.test(value) ? `"${value.replace(/"/g, '""')}"` : value;
}

/** Joins a header + rows into a CSV string (cells escaped, `\r\n` line endings). */
export function toCsv(headers: string[], rows: string[][]): string {
	return [headers, ...rows].map((row) => row.map(escapeCsvField).join(',')).join('\r\n');
}

/** CSV of the window's candles: `ts,open,high,low,close,volume`. */
export function candlesToCsv(candles: Candle[]): string {
	return toCsv(
		['ts', 'open', 'high', 'low', 'close', 'volume'],
		candles.map((c) => [c.ts, String(c.o), String(c.h), String(c.l), String(c.c), String(c.v)])
	);
}

/** CSV of the window's decisions; the opaque snapshot is serialised as JSON. */
export function decisionsToCsv(decisions: LiveDecision[]): string {
	return toCsv(
		['decision_id', 'created_at', 'reason', 'orders_count', 'session_id', 'market_context'],
		decisions.map((d) => [
			d.decision_id,
			d.created_at,
			d.reason ?? '',
			String(d.orders_count),
			d.session_id,
			d.market_context == null ? '' : JSON.stringify(d.market_context)
		])
	);
}

/** Clicks a transient anchor to download `url` as `filename`. Browser-only. */
function triggerDownload(url: string, filename: string): void {
	const a = document.createElement('a');
	a.href = url;
	a.download = filename;
	document.body.appendChild(a);
	a.click();
	a.remove();
}

/** Downloads `text` as a file (default CSV). Browser-only. */
export function downloadText(filename: string, text: string, type = 'text/csv'): void {
	const url = URL.createObjectURL(new Blob([text], { type: `${type};charset=utf-8` }));
	triggerDownload(url, filename);
	// Revoke on the next tick so the download has grabbed the URL.
	setTimeout(() => URL.revokeObjectURL(url), 0);
}

/** Downloads a canvas as a PNG. Browser-only. */
export function downloadCanvasPng(canvas: HTMLCanvasElement, filename: string): void {
	triggerDownload(canvas.toDataURL('image/png'), filename);
}

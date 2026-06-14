/**
 * Builds indicator sub-chart series from live decision snapshots (#24).
 *
 * Same data source as the backtest overlays (#23): the opaque
 * `decision_market_context` snapshot. Rather than hard-coding indicator names
 * (DMI/RSI/ADX/ATR…) — whose exact keys are owned by `robot_rust` and unknown
 * here — we surface **every numeric top-level field generically**, one line per
 * key. The named target indicators therefore appear whenever the snapshot
 * carries them; an empty/absent snapshot yields no series (clean degradation).
 *
 * Each series is rendered in its own pane below the price chart, so unlike the
 * price-scale overlays these carry their own scale. Points are sparse (one per
 * decision), mirroring the overlays — a continuous series would require
 * recomputation from candles, out of scope for this lot.
 *
 * Pure and DOM-free so it is unit-testable in the node project.
 */
import type { LiveDecision } from '$lib/api/types';
import type { ChartOverlay, OverlayPoint } from '$lib/chart/overlays';

// Deterministic, colour-blind-friendly palette assigned by sorted indicator key
// (kept in sync with the backtest overlay palette for visual consistency).
const INDICATOR_PALETTE = [
	'#3b82f6', // blue
	'#f59e0b', // amber
	'#10b981', // emerald
	'#ec4899', // pink
	'#8b5cf6', // violet
	'#14b8a6', // teal
	'#ef4444', // red
	'#84cc16' // lime
];

/**
 * One indicator line per numeric top-level field of the decisions'
 * `market_context`, keyed/coloured deterministically by sorted key.
 */
export function decisionsToIndicators(decisions: LiveDecision[]): ChartOverlay[] {
	// Collect points per numeric snapshot key, in chronological order per key.
	const points: Record<string, OverlayPoint[]> = {};

	for (const decision of decisions) {
		const snapshot = decision.market_context as Record<string, unknown> | null | undefined;
		if (!snapshot) continue;
		for (const [key, value] of Object.entries(snapshot)) {
			if (typeof value !== 'number' || !Number.isFinite(value)) continue;
			(points[key] ??= []).push({ ts: decision.created_at, value });
		}
	}

	// Sort keys alphabetically so ids and colours are stable across renders.
	const keys = Object.keys(points).sort((a, b) => a.localeCompare(b));
	return keys.map((key, i) => ({
		id: key,
		title: key,
		color: INDICATOR_PALETTE[i % INDICATOR_PALETTE.length],
		data: points[key]
	}));
}

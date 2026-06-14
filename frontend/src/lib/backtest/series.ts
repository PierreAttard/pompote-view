/**
 * Pure mappers turning backtest API rows into the source-agnostic shapes the
 * `<Chart>` component consumes (`ChartMarker`, `ChartOverlay`). Kept free of
 * Svelte/DOM so they are unit-testable in the node project.
 */
import type { BacktestOrder, Decision } from '$lib/api/types';
import type { ChartMarker } from '$lib/chart/markers';
import type { ChartOverlay, OverlayPoint } from '$lib/chart/overlays';

/** Map orders to buy/sell chart markers (`B`/`S` labels). */
export function ordersToMarkers(orders: BacktestOrder[]): ChartMarker[] {
	return orders.map((o) => {
		// Defensive: API rows can contradict the `string` type at runtime.
		const side = String(o.side ?? '').toLowerCase();
		return {
			ts: o.market_ts,
			side: o.side,
			text: side === 'buy' ? 'B' : side === 'sell' ? 'S' : '?'
		};
	});
}

// Deterministic, colour-blind-friendly palette assigned by sorted overlay key.
const OVERLAY_PALETTE = [
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
 * Build one overlay line per numeric top-level field of the decision
 * `snapshot`. The snapshot is opaque (`jsonb` owned by robot_rust), so rather
 * than hard-coding SMA/EMA/Bollinger key names we surface every scalar number
 * generically; the user enables the ones they want via the toggles. Nested or
 * non-numeric fields are ignored, so an empty/absent snapshot yields no
 * overlays (clean degradation).
 */
export function decisionsToOverlays(decisions: Decision[]): ChartOverlay[] {
	// Collect points per numeric snapshot key, in market order per key.
	const points: Record<string, OverlayPoint[]> = {};

	for (const decision of decisions) {
		const snapshot = decision.snapshot as Record<string, unknown> | null | undefined;
		if (!snapshot) continue;
		for (const [key, value] of Object.entries(snapshot)) {
			if (typeof value !== 'number' || !Number.isFinite(value)) continue;
			(points[key] ??= []).push({ ts: decision.market_ts, value });
		}
	}

	// Sort keys alphabetically so overlay ids and colours are deterministic.
	const keys = Object.keys(points).sort((a, b) => a.localeCompare(b));
	return keys.map((key, i) => ({
		id: key,
		title: key,
		color: OVERLAY_PALETTE[i % OVERLAY_PALETTE.length],
		data: points[key]
	}));
}

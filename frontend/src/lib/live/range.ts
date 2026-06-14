/**
 * Pure helpers for the live monitoring time-range selector.
 *
 * Kept free of Svelte/DOM so the preset maths is unit-testable. The selectors
 * persist their state in the URL query string; these helpers compute the
 * `[from, to)` window for the quick presets.
 */

/** Quick range presets offered by the selector. */
export type RangePreset = '1h' | '6h' | '24h' | '7d';

/** Ordered presets with human labels (finest to coarsest). */
export const RANGE_PRESETS: { id: RangePreset; label: string }[] = [
	{ id: '1h', label: 'Dernière heure' },
	{ id: '6h', label: '6 heures' },
	{ id: '24h', label: '24 heures' },
	{ id: '7d', label: '7 jours' }
];

const PRESET_MS: Record<RangePreset, number> = {
	'1h': 60 * 60 * 1000,
	'6h': 6 * 60 * 60 * 1000,
	'24h': 24 * 60 * 60 * 1000,
	'7d': 7 * 24 * 60 * 60 * 1000
};

/** `true` if `value` is a known preset id. */
export function isRangePreset(value: string | null | undefined): value is RangePreset {
	return value === '1h' || value === '6h' || value === '24h' || value === '7d';
}

/** Duration of a preset window in milliseconds. */
export function presetMs(preset: RangePreset): number {
	return PRESET_MS[preset];
}

/**
 * Computes the `[from, to)` window (RFC3339) for a preset relative to `now`.
 * `to` is `now`; `from` is `now - preset`.
 */
export function presetWindow(
	preset: RangePreset,
	now: Date = new Date()
): {
	from: string;
	to: string;
} {
	const to = now;
	const from = new Date(now.getTime() - PRESET_MS[preset]);
	return { from: from.toISOString(), to: to.toISOString() };
}

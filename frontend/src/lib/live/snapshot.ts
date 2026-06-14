/**
 * Renders the opaque `decision_market_context` snapshot into a flat, readable
 * list of label/value rows for the decision tooltip.
 *
 * The snapshot is free-form JSONB owned by `robot_rust`, so rather than
 * hard-coding indicator names (DMI/RSI/Bollinger/Ichimoku…) we flatten it
 * generically: every scalar becomes a row, nested objects are walked up to a
 * small depth with dotted labels, and anything unrepresentable is skipped. An
 * empty/absent snapshot yields no rows (clean degradation).
 *
 * Pure and DOM-free so it is unit-testable in the node project.
 */

/** One displayable row of the snapshot. */
export interface SnapshotRow {
	/** Dotted key path, e.g. `dmi.plus_di`. */
	label: string;
	/** Human-formatted scalar value. */
	value: string;
}

/** Formats a scalar, trimming floating-point noise on non-integer numbers. */
function formatScalar(value: number | string | boolean): string {
	if (typeof value === 'number') {
		if (!Number.isFinite(value)) return String(value);
		if (Number.isInteger(value)) return String(value);
		// Round to 4 decimals then drop trailing zeros (e.g. 1.23400 -> "1.234").
		return String(Number(value.toFixed(4)));
	}
	return String(value);
}

function walk(node: unknown, path: string[], rows: SnapshotRow[], depth: number): void {
	if (node === null || node === undefined) return;

	if (typeof node === 'number' || typeof node === 'string' || typeof node === 'boolean') {
		rows.push({ label: path.join('.'), value: formatScalar(node) });
		return;
	}

	if (Array.isArray(node)) {
		// Arrays are rare in a snapshot; surface scalar arrays compactly and skip
		// the rest rather than exploding the row count.
		const scalars = node.filter(
			(v) => typeof v === 'number' || typeof v === 'string' || typeof v === 'boolean'
		);
		if (scalars.length === node.length && node.length > 0) {
			rows.push({ label: path.join('.'), value: scalars.map(formatScalar).join(', ') });
		}
		return;
	}

	if (typeof node === 'object') {
		if (depth <= 0) return;
		for (const [key, value] of Object.entries(node as Record<string, unknown>)) {
			walk(value, [...path, key], rows, depth - 1);
		}
	}
}

/**
 * Flattens a snapshot object into ordered rows. `maxDepth` bounds nested-object
 * recursion (default 2: top-level groups + their direct scalar fields).
 */
export function flattenSnapshot(snapshot: unknown, maxDepth = 2): SnapshotRow[] {
	const rows: SnapshotRow[] = [];
	if (snapshot && typeof snapshot === 'object' && !Array.isArray(snapshot)) {
		walk(snapshot, [], rows, maxDepth);
	}
	return rows;
}

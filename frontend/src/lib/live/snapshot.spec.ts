import { describe, expect, it } from 'vitest';
import { flattenSnapshot } from './snapshot';

describe('flattenSnapshot', () => {
	it('returns an empty list for null/undefined/non-object snapshots', () => {
		expect(flattenSnapshot(null)).toEqual([]);
		expect(flattenSnapshot(undefined)).toEqual([]);
		expect(flattenSnapshot(42)).toEqual([]);
		expect(flattenSnapshot('rsi')).toEqual([]);
		expect(flattenSnapshot([1, 2, 3])).toEqual([]);
	});

	it('flattens top-level scalars, preserving key order', () => {
		expect(flattenSnapshot({ rsi: 55, trend: 'up', crossed: true })).toEqual([
			{ label: 'rsi', value: '55' },
			{ label: 'trend', value: 'up' },
			{ label: 'crossed', value: 'true' }
		]);
	});

	it('trims floating-point noise on non-integer numbers', () => {
		expect(flattenSnapshot({ atr: 1.2300000001 })).toEqual([{ label: 'atr', value: '1.23' }]);
		expect(flattenSnapshot({ k: 0.1 + 0.2 })).toEqual([{ label: 'k', value: '0.3' }]);
	});

	it('walks nested objects with dotted labels up to the depth bound', () => {
		const snapshot = { dmi: { plus_di: 21.5, minus_di: 18 }, rsi: 60 };
		expect(flattenSnapshot(snapshot)).toEqual([
			{ label: 'dmi.plus_di', value: '21.5' },
			{ label: 'dmi.minus_di', value: '18' },
			{ label: 'rsi', value: '60' }
		]);
	});

	it('does not recurse past maxDepth', () => {
		const deep = { a: { b: { c: 1 } } };
		// Default depth 2 stops before reaching `c`.
		expect(flattenSnapshot(deep)).toEqual([]);
		// Depth 3 reaches it.
		expect(flattenSnapshot(deep, 3)).toEqual([{ label: 'a.b.c', value: '1' }]);
	});

	it('renders a scalar array compactly and skips object arrays', () => {
		expect(flattenSnapshot({ bands: [1, 2, 3] })).toEqual([{ label: 'bands', value: '1, 2, 3' }]);
		expect(flattenSnapshot({ rows: [{ x: 1 }] })).toEqual([]);
	});

	it('skips null/undefined leaf values', () => {
		expect(flattenSnapshot({ a: null, b: 1 })).toEqual([{ label: 'b', value: '1' }]);
	});
});

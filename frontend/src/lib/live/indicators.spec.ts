import { describe, expect, it } from 'vitest';
import type { LiveDecision } from '$lib/api/types';
import { decisionsToIndicators } from './indicators';

function decision(overrides: Partial<LiveDecision> = {}): LiveDecision {
	return {
		decision_id: 'd1',
		session_id: 's1',
		created_at: '2026-06-01T00:00:00Z',
		reason: 'r',
		orders_count: 0,
		market_context: null,
		...overrides
	};
}

// `market_context` is opaque JSONB in the generated types; cast samples.
function ctx(value: Record<string, unknown>): LiveDecision['market_context'] {
	return value as unknown as LiveDecision['market_context'];
}

describe('decisionsToIndicators', () => {
	it('builds one line per numeric top-level field, sorted by key, with the palette', () => {
		const indicators = decisionsToIndicators([
			decision({ created_at: '2026-06-01T00:00:00Z', market_context: ctx({ rsi: 28, adx: 30 }) })
		]);
		expect(indicators.map((i) => i.id)).toEqual(['adx', 'rsi']); // alphabetical
		expect(indicators[0]).toMatchObject({ id: 'adx', title: 'adx', color: '#3b82f6' });
		expect(indicators[1].color).toBe('#f59e0b');
		expect(indicators[1].data).toEqual([{ ts: '2026-06-01T00:00:00Z', value: 28 }]);
	});

	it('accumulates a point per decision, per key, using created_at as the timestamp', () => {
		const indicators = decisionsToIndicators([
			decision({ created_at: '2026-06-01T00:00:00Z', market_context: ctx({ rsi: 28 }) }),
			decision({ created_at: '2026-06-01T01:00:00Z', market_context: ctx({ rsi: 35 }) })
		]);
		expect(indicators).toHaveLength(1);
		expect(indicators[0].data).toEqual([
			{ ts: '2026-06-01T00:00:00Z', value: 28 },
			{ ts: '2026-06-01T01:00:00Z', value: 35 }
		]);
	});

	it('ignores non-numeric, non-finite, nested and null fields', () => {
		const indicators = decisionsToIndicators([
			decision({
				market_context: ctx({
					rsi: 42,
					label: 'bull',
					bad: Number.NaN,
					inf: Number.POSITIVE_INFINITY,
					dmi: { plus_di: 25 }, // nested object: not surfaced
					missing: null
				})
			})
		]);
		expect(indicators.map((i) => i.id)).toEqual(['rsi']);
	});

	it('yields no series for empty/absent snapshots (clean degradation)', () => {
		expect(decisionsToIndicators([])).toEqual([]);
		expect(decisionsToIndicators([decision({ market_context: null })])).toEqual([]);
		expect(decisionsToIndicators([decision({ market_context: ctx({}) })])).toEqual([]);
	});
});

import { describe, expect, it } from 'vitest';
import { nextQuery } from './url';

function q(s: string): URLSearchParams {
	return new URLSearchParams(s);
}

describe('nextQuery', () => {
	it('sets a new param, preserving the others', () => {
		const out = nextQuery(q('strategy=a&symbol=BTC'), { timeframe: '1h' });
		expect(new URLSearchParams(out).get('timeframe')).toBe('1h');
		expect(new URLSearchParams(out).get('strategy')).toBe('a');
	});

	it('replaces an existing param', () => {
		const out = nextQuery(q('mode=all'), { mode: 'paper' });
		expect(out).toBe('mode=paper');
	});

	it('clears a param when value is null or empty', () => {
		expect(nextQuery(q('strategy=a&mode=live'), { strategy: null }).includes('strategy')).toBe(
			false
		);
		expect(nextQuery(q('symbol=BTC'), { symbol: '' })).toBe('');
	});

	it('applies several changes at once (e.g. mode + orphan strategy clear)', () => {
		const out = nextQuery(q('mode=all&strategy=a&symbol=BTC'), { mode: 'live', strategy: null });
		const p = new URLSearchParams(out);
		expect(p.get('mode')).toBe('live');
		expect(p.has('strategy')).toBe(false);
		expect(p.get('symbol')).toBe('BTC');
	});
});

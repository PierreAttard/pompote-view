import { describe, expect, it } from 'vitest';
import type { Candle, LiveDecision } from '$lib/api/types';
import { candlesToCsv, decisionsToCsv, escapeCsvField, toCsv } from './export';

describe('escapeCsvField', () => {
	it('leaves plain values untouched', () => {
		expect(escapeCsvField('hello')).toBe('hello');
		expect(escapeCsvField('42.5')).toBe('42.5');
	});

	it('quotes and doubles quotes when a field contains a delimiter/quote/newline', () => {
		expect(escapeCsvField('a,b')).toBe('"a,b"');
		expect(escapeCsvField('say "hi"')).toBe('"say ""hi"""');
		expect(escapeCsvField('line1\nline2')).toBe('"line1\nline2"');
	});
});

describe('toCsv', () => {
	it('joins header + rows with CRLF and escaped cells', () => {
		expect(toCsv(['a', 'b'], [['1', '2,3']])).toBe('a,b\r\n1,"2,3"');
	});
});

describe('candlesToCsv', () => {
	it('emits the OHLCV header and one row per candle', () => {
		const candles: Candle[] = [
			{ ts: '2026-06-01T00:00:00Z', o: 100, h: 110, l: 95, c: 105, v: 12 }
		];
		expect(candlesToCsv(candles)).toBe(
			'ts,open,high,low,close,volume\r\n2026-06-01T00:00:00Z,100,110,95,105,12'
		);
	});
});

describe('decisionsToCsv', () => {
	function decision(overrides: Partial<LiveDecision> = {}): LiveDecision {
		return {
			decision_id: 'd1',
			created_at: '2026-06-01T00:00:00Z',
			reason: 'plain reason',
			orders_count: 1,
			session_id: 's1',
			market_context: { rsi: 28 } as unknown as LiveDecision['market_context'],
			...overrides
		};
	}

	it('serialises the snapshot as JSON and escapes commas in the reason', () => {
		const csv = decisionsToCsv([decision({ reason: 'RSI oversold, DMI cross' })]);
		const [header, row] = csv.split('\r\n');
		expect(header).toBe('decision_id,created_at,reason,orders_count,session_id,market_context');
		expect(row).toContain('"RSI oversold, DMI cross"');
		expect(row).toContain('"{""rsi"":28}"');
	});

	it('renders an empty cell for an absent snapshot', () => {
		const row = decisionsToCsv([decision({ market_context: null })]).split('\r\n')[1];
		// Trailing empty market_context cell.
		expect(row.endsWith(',')).toBe(true);
	});
});

import { describe, expect, it } from 'vitest';
import { formatAge, formatNumber, formatTimestamp } from './format';

describe('formatTimestamp', () => {
	it('formats an RFC3339 timestamp in UTC (no local-time shift)', () => {
		expect(formatTimestamp('2026-06-01T00:30:00Z')).toBe('01/06/2026 00:30:00');
	});

	it('keeps the wall-clock in UTC regardless of an explicit offset', () => {
		// 02:30+02:00 == 00:30 UTC.
		expect(formatTimestamp('2026-06-01T02:30:00+02:00')).toBe('01/06/2026 00:30:00');
	});

	it('returns the raw string when it cannot be parsed', () => {
		expect(formatTimestamp('not-a-date')).toBe('not-a-date');
	});
});

describe('formatNumber', () => {
	it('renders integers as-is', () => {
		expect(formatNumber(42)).toBe('42');
	});

	it('trims trailing zeros on decimals', () => {
		expect(formatNumber(100.5)).toBe('100.5');
		expect(formatNumber(0.123)).toBe('0.123');
	});

	it('caps precision at 8 decimals', () => {
		expect(formatNumber(0.123456789)).toBe('0.12345679');
	});

	it('renders an em dash for null/undefined/non-finite', () => {
		expect(formatNumber(null)).toBe('—');
		expect(formatNumber(undefined)).toBe('—');
		expect(formatNumber(Number.NaN)).toBe('—');
		expect(formatNumber(Number.POSITIVE_INFINITY)).toBe('—');
	});
});

describe('formatAge', () => {
	it('reads "à l\'instant" under a second (and for negative ages)', () => {
		expect(formatAge(0)).toBe("à l'instant");
		expect(formatAge(900)).toBe("à l'instant");
		expect(formatAge(-1000)).toBe("à l'instant");
	});

	it('formats seconds, minutes and hours', () => {
		expect(formatAge(5_000)).toBe('il y a 5 s');
		expect(formatAge(59_000)).toBe('il y a 59 s');
		expect(formatAge(60_000)).toBe('il y a 1 min');
		expect(formatAge(90_000)).toBe('il y a 1 min');
		expect(formatAge(3_600_000)).toBe('il y a 1 h');
	});
});

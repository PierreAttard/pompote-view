import { describe, expect, it } from 'vitest';
import {
	MAX_CANDLE_POINTS,
	bucketCount,
	depthStatus,
	suggestTimeframe,
	timeframeSeconds
} from './depth';

// The backend's allowed set, finest → coarsest, as the timeframes endpoint
// returns it.
const TIMEFRAMES = [
	'5s',
	'15s',
	'30s',
	'1m',
	'3m',
	'5m',
	'15m',
	'30m',
	'1h',
	'2h',
	'4h',
	'6h',
	'12h',
	'1d'
];

describe('timeframeSeconds', () => {
	it('parses each unit', () => {
		expect(timeframeSeconds('5s')).toBe(5);
		expect(timeframeSeconds('1m')).toBe(60);
		expect(timeframeSeconds('4h')).toBe(4 * 3600);
		expect(timeframeSeconds('1d')).toBe(86400);
	});

	it('returns null for an unrecognised shape', () => {
		expect(timeframeSeconds('bogus')).toBeNull();
		expect(timeframeSeconds('1w')).toBeNull();
		expect(timeframeSeconds('0m')).toBeNull();
		expect(timeframeSeconds('')).toBeNull();
	});
});

describe('bucketCount', () => {
	it('divides the window by the timeframe width', () => {
		// 24h / 1h = 24 buckets.
		expect(bucketCount('24h', '1h')).toBe(24);
		// 24h / 5s = 17280 buckets.
		expect(bucketCount('24h', '5s')).toBe(17280);
		// 1h / 1m = 60 buckets.
		expect(bucketCount('1h', '1m')).toBe(60);
	});

	it('uses ceiling division like the backend', () => {
		// 1h = 3600s over a 7m (420s) width → 8.57 → 9 buckets.
		expect(bucketCount('1h', '7m' as string)).toBe(9);
	});

	it('returns null for an unparseable timeframe', () => {
		expect(bucketCount('24h', 'bogus')).toBeNull();
	});
});

describe('depthStatus', () => {
	it('does not flag a selection within the cap', () => {
		const status = depthStatus('24h', '1h', TIMEFRAMES);
		expect(status.count).toBe(24);
		expect(status.exceeds).toBe(false);
		expect(status.suggestion).toBeNull();
	});

	it('flags an over-cap selection and suggests the finest timeframe that fits', () => {
		// 7d / 1m = 10080 > 5000.
		const status = depthStatus('7d', '1m', TIMEFRAMES);
		expect(status.exceeds).toBe(true);
		expect(status.count).toBeGreaterThan(MAX_CANDLE_POINTS);
		// 7d needs ≥ ~121s/bucket; 3m (180s) → 3360 buckets is the finest that fits.
		expect(status.suggestion).toBe('3m');
		expect(bucketCount('7d', status.suggestion!)).toBeLessThanOrEqual(MAX_CANDLE_POINTS);
	});

	it('never blocks when the timeframe is unparseable', () => {
		const status = depthStatus('7d', 'bogus', TIMEFRAMES);
		expect(status.exceeds).toBe(false);
	});
});

describe('suggestTimeframe', () => {
	it('returns null when no offered timeframe fits the window', () => {
		// 7d at 5s only → 120960 buckets, nothing else to fall back to.
		expect(suggestTimeframe('7d', ['5s'])).toBeNull();
	});
});

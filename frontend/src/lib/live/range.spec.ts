import { describe, expect, it } from 'vitest';
import { isRangePreset, presetWindow, RANGE_PRESETS } from './range';

describe('range presets', () => {
	it('exposes the four presets finest to coarsest', () => {
		expect(RANGE_PRESETS.map((p) => p.id)).toEqual(['1h', '6h', '24h', '7d']);
	});

	it('recognises valid preset ids', () => {
		expect(isRangePreset('24h')).toBe(true);
		expect(isRangePreset('foo')).toBe(false);
		expect(isRangePreset(null)).toBe(false);
	});

	it('computes a window ending at now, starting preset-duration earlier', () => {
		const now = new Date('2026-06-01T12:00:00.000Z');
		expect(presetWindow('1h', now)).toEqual({
			from: '2026-06-01T11:00:00.000Z',
			to: '2026-06-01T12:00:00.000Z'
		});
		expect(presetWindow('7d', now).from).toBe('2026-05-25T12:00:00.000Z');
	});
});

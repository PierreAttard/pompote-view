import { page } from 'vitest/browser';
import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import DecisionTooltip from './DecisionTooltip.svelte';
import type { LiveDecision } from '$lib/api/types';

function decision(overrides: Partial<LiveDecision> = {}): LiveDecision {
	return {
		decision_id: 'd1',
		created_at: '2026-06-01T00:00:00Z',
		reason: 'RSI oversold, DMI bullish cross',
		orders_count: 1,
		session_id: 's1',
		// `market_context` is opaque JSONB in the generated types; cast the sample.
		market_context: { rsi: 28.5, dmi: { plus_di: 25, minus_di: 12 } } as unknown as Record<
			string,
			never
		>,
		...overrides
	};
}

describe('DecisionTooltip.svelte', () => {
	it('renders the reason and the flattened snapshot rows', async () => {
		render(DecisionTooltip, { decision: decision() });
		await expect
			.element(page.getByTestId('decision-reason'))
			.toHaveTextContent('RSI oversold, DMI bullish cross');
		await expect.element(page.getByText('dmi.plus_di')).toBeInTheDocument();
		await expect.element(page.getByText('28.5')).toBeInTheDocument();
		await expect.element(page.getByText('25', { exact: true })).toBeInTheDocument();
	});

	it('shows a fallback when the snapshot is empty', async () => {
		render(DecisionTooltip, { decision: decision({ market_context: null }) });
		await expect.element(page.getByText(/Pas de snapshot/)).toBeInTheDocument();
	});

	it('renders an em dash when the reason is blank', async () => {
		render(DecisionTooltip, { decision: decision({ reason: '' }) });
		await expect.element(page.getByTestId('decision-reason')).toHaveTextContent('—');
	});
});

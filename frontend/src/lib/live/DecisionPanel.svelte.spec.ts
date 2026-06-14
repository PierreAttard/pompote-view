import { page } from 'vitest/browser';
import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import DecisionPanel from './DecisionPanel.svelte';
import type { LiveDecision, LiveFill, Order } from '$lib/api/types';

function decision(overrides: Partial<LiveDecision> = {}): LiveDecision {
	return {
		decision_id: 'dec-123',
		session_id: 'sess-9',
		created_at: '2026-06-01T00:30:00Z',
		reason: 'RSI oversold, DMI bullish cross',
		orders_count: 1,
		// `market_context` is opaque JSONB in the generated types; cast the sample.
		market_context: { rsi: 28.5, dmi: { plus_di: 25 } } as unknown as Record<string, never>,
		...overrides
	};
}

function order(overrides: Partial<Order> = {}): Order {
	return {
		order_id: 'o1',
		decision_id: 'dec-123',
		side: 'buy',
		quantity: 1.5,
		price: 100,
		status: 'filled',
		created_at: '2026-06-01T00:30:00Z',
		...overrides
	};
}

function fill(overrides: Partial<LiveFill> = {}): LiveFill {
	return {
		fill_id: 'f1',
		order_id: 'o1',
		side: 'buy',
		price: 100.25,
		quantity: 1.5,
		fee: 0.1,
		fee_asset: 'USDT',
		executed_at: '2026-06-01T00:30:01Z',
		is_paper: true,
		...overrides
	};
}

function base() {
	return { decision: decision(), orders: [order()], fills: [fill()], onClose: vi.fn() };
}

describe('DecisionPanel.svelte', () => {
	it('renders the reason and the decision metadata', async () => {
		render(DecisionPanel, base());
		await expect
			.element(page.getByTestId('panel-reason'))
			.toHaveTextContent('RSI oversold, DMI bullish cross');
		// created_at formatted in UTC.
		await expect.element(page.getByText('01/06/2026 00:30:00 UTC')).toBeInTheDocument();
		await expect.element(page.getByText('dec-123')).toBeInTheDocument();
		await expect.element(page.getByText('sess-9')).toBeInTheDocument();
	});

	it('renders the flattened snapshot and the raw JSON debug view', async () => {
		render(DecisionPanel, base());
		// Scope to the flattened list: the value also appears in the raw JSON below.
		const snapshot = page.getByTestId('panel-snapshot');
		await expect.element(snapshot).toHaveTextContent('dmi.plus_di');
		await expect.element(snapshot).toHaveTextContent('28.5');
		await expect.element(page.getByTestId('panel-snapshot-json')).toHaveTextContent('"rsi": 28.5');
	});

	it('shows a fallback when the snapshot is empty', async () => {
		render(DecisionPanel, { ...base(), decision: decision({ market_context: null }) });
		await expect.element(page.getByText(/Pas de snapshot/)).toBeInTheDocument();
	});

	it('lists the associated orders', async () => {
		render(DecisionPanel, {
			...base(),
			orders: [order({ order_id: 'o1', side: 'buy', status: 'filled', quantity: 1.5, price: 100 })]
		});
		const orders = page.getByTestId('panel-orders');
		await expect.element(orders).toHaveTextContent('Ordres (1)');
		await expect.element(orders).toHaveTextContent('filled');
		await expect.element(orders).toHaveTextContent('1.5');
	});

	it('marks each fill paper or live', async () => {
		render(DecisionPanel, {
			...base(),
			fills: [
				fill({ fill_id: 'f1', is_paper: true }),
				fill({ fill_id: 'f2', order_id: 'o1', is_paper: false })
			]
		});
		const fills = page.getByTestId('panel-fills');
		await expect.element(fills).toHaveTextContent('paper');
		await expect.element(fills).toHaveTextContent('live');
	});

	it('shows a loading hint while the fills query is pending', async () => {
		render(DecisionPanel, { ...base(), fills: [], fillsPending: true });
		await expect.element(page.getByText(/Chargement des exécutions/)).toBeInTheDocument();
	});

	it('shows an unavailable hint when the fills query failed', async () => {
		render(DecisionPanel, { ...base(), fills: [], fillsError: true });
		await expect.element(page.getByText(/Exécutions indisponibles/)).toBeInTheDocument();
	});

	it('shows an empty hint when there are no fills', async () => {
		render(DecisionPanel, { ...base(), fills: [] });
		await expect.element(page.getByText(/Aucune exécution/)).toBeInTheDocument();
	});

	it('closes via the close button', async () => {
		const onClose = vi.fn();
		render(DecisionPanel, { ...base(), onClose });
		await page.getByTestId('decision-panel-close').click();
		expect(onClose).toHaveBeenCalledOnce();
	});

	it('closes on the Escape key', async () => {
		const onClose = vi.fn();
		render(DecisionPanel, { ...base(), onClose });
		await expect.element(page.getByTestId('decision-panel')).toBeInTheDocument();
		window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
		expect(onClose).toHaveBeenCalledOnce();
	});

	it('hides the stepper when there is a single decision on the candle', async () => {
		render(DecisionPanel, { ...base(), position: 1, total: 1 });
		await expect.element(page.getByTestId('panel-stepper')).not.toBeInTheDocument();
	});

	it('steps between decisions sharing the candle', async () => {
		const onPrev = vi.fn();
		const onNext = vi.fn();
		render(DecisionPanel, { ...base(), position: 2, total: 3, onPrev, onNext });
		await expect.element(page.getByTestId('panel-position')).toHaveTextContent('2/3');
		await page.getByTestId('panel-prev').click();
		await page.getByTestId('panel-next').click();
		expect(onPrev).toHaveBeenCalledOnce();
		expect(onNext).toHaveBeenCalledOnce();
	});
});

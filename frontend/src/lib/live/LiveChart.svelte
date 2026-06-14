<script lang="ts">
	import { createQuery } from '@tanstack/svelte-query';
	import type { LiveDecision } from '$lib/api/types';
	import Chart, { type HoverInfo } from '$lib/chart/Chart.svelte';
	import IndicatorToggles from '$lib/chart/IndicatorToggles.svelte';
	import {
		buildDecisionLookup,
		decisionsQueryKey,
		fetchDecisions,
		fetchFills,
		fetchOrders,
		fillsForOrders,
		fillsQueryKey,
		ordersForDecision,
		ordersQueryKey,
		ordersToMarkers,
		type AnnotationParams
	} from './annotations';
	import { candlesQueryKey, fetchCandles, type CandlesParams } from './candles';
	import DecisionPanel from './DecisionPanel.svelte';
	import DecisionTooltip from './DecisionTooltip.svelte';
	import { timeframeSeconds } from './depth';
	import { decisionsToIndicators } from './indicators';
	import VolumeProfile from './VolumeProfile.svelte';

	interface Props extends CandlesParams {
		/** Strategy whose orders/decisions annotate the chart. */
		strategyId: string;
	}

	// The selectors flow in as props; the page derives the `[from, to)` window.
	let { strategyId, exchange, symbol, timeframe, from, to }: Props = $props();

	const params = $derived<CandlesParams>({ exchange, symbol, timeframe, from, to });
	const annotationParams = $derived<AnnotationParams>({ strategyId, from, to });

	// Candles drive the chart's loading/error/empty/chart states. The query key is
	// derived from the params, so changing any selector refetches.
	const candlesQuery = createQuery(() => ({
		queryKey: candlesQueryKey(params),
		queryFn: () => fetchCandles(params)
	}));

	// Orders (buy/sell markers) and decisions (reason + snapshot) are best-effort
	// annotations: if they fail the chart still renders, just without markers.
	const ordersQuery = createQuery(() => ({
		queryKey: ordersQueryKey(annotationParams),
		queryFn: () => fetchOrders(annotationParams),
		enabled: strategyId !== ''
	}));
	const decisionsQuery = createQuery(() => ({
		queryKey: decisionsQueryKey(annotationParams),
		queryFn: () => fetchDecisions(annotationParams),
		enabled: strategyId !== ''
	}));

	const candles = $derived(candlesQuery.data ?? []);
	const orders = $derived(ordersQuery.data ?? []);
	const decisions = $derived(decisionsQuery.data ?? []);

	const markers = $derived(ordersToMarkers(orders));
	// Bucket-aligned lookup so a crosshair landing on a bar resolves to the
	// decisions whose orders fall in that candle.
	const lookup = $derived(buildDecisionLookup(orders, decisions, timeframeSeconds(timeframe) ?? 0));

	// Indicator sub-charts (#24): one toggleable line per numeric snapshot field,
	// extracted generically from the decisions. Off by default — the user enables
	// the ones they want; only enabled indicators get a pane (keeps it light).
	const indicators = $derived(decisionsToIndicators(decisions));
	let indicatorVisible = $state<Record<string, boolean>>({});
	const enabledSubcharts = $derived(indicators.filter((ind) => indicatorVisible[ind.id]));

	// Decision detail panel (#22). A click selects a candle bucket; we keep the
	// bucket key + index (not the decision object) so the panel stays correct when
	// the underlying queries refetch (live polling, #26).
	let selected = $state<{ bucket: number; index: number } | null>(null);

	// Fills back the panel's executions table. Fetched lazily — only once a
	// decision is selected — then cached per window and reused across decisions.
	const fillsQuery = createQuery(() => ({
		queryKey: fillsQueryKey(annotationParams),
		queryFn: () => fetchFills(annotationParams),
		enabled: strategyId !== '' && selected !== null
	}));
	const fills = $derived(fillsQuery.data ?? []);

	const selectedList = $derived(selected ? (lookup.get(selected.bucket) ?? []) : []);
	const selectedIndex = $derived(
		selectedList.length === 0 ? 0 : Math.min(selected?.index ?? 0, selectedList.length - 1)
	);
	const selectedDecision = $derived(selectedList[selectedIndex] ?? null);
	const panelOrders = $derived(
		selectedDecision ? ordersForDecision(orders, selectedDecision.decision_id) : []
	);
	const panelFills = $derived(fillsForOrders(fills, panelOrders));

	// Drop a stale selection when its bucket no longer holds decisions (selector
	// change, or a future refetch that removes them).
	$effect(() => {
		if (selected && selectedList.length === 0) selected = null;
	});

	function selectBucket(info: HoverInfo | null): void {
		if (!info) return; // click off the data area: leave the panel as-is
		const list = lookup.get(Number(info.time));
		if (list && list.length > 0) selected = { bucket: Number(info.time), index: 0 };
	}

	function stepPanel(delta: number): void {
		if (!selected || selectedList.length === 0) return;
		const len = selectedList.length;
		selected = { bucket: selected.bucket, index: (selectedIndex + delta + len) % len };
	}

	let containerWidth = $state(0);
	let hovered = $state<{ decision: LiveDecision; extra: number; x: number; y: number } | null>(
		null
	);

	const TOOLTIP_WIDTH = 256; // matches DecisionTooltip's w-64

	function handleHover(info: HoverInfo | null): void {
		if (!info) {
			hovered = null;
			return;
		}
		const list = lookup.get(Number(info.time));
		hovered =
			list && list.length > 0
				? { decision: list[0], extra: list.length - 1, x: info.x, y: info.y }
				: null;
	}

	// Flip the tooltip to the left of the cursor when it would overflow the right
	// edge; keep a small margin from the container edges.
	const tooltipLeft = $derived.by(() => {
		if (!hovered) return 0;
		const right = hovered.x + 12 + TOOLTIP_WIDTH;
		return right > containerWidth ? Math.max(8, hovered.x - TOOLTIP_WIDTH - 12) : hovered.x + 12;
	});
	const tooltipTop = $derived(hovered ? hovered.y + 12 : 0);
</script>

<div class="space-y-2">
	{#if indicators.length > 0}
		<!-- Indicator sub-chart selector (#24): toggles which snapshot indicators
		     get their own pane below the price chart. -->
		<div data-testid="live-indicator-toggles">
			<IndicatorToggles {indicators} bind:visible={indicatorVisible} />
		</div>
	{/if}
	<div class="flex items-stretch gap-2">
		<div
			class="relative min-h-[280px] flex-1 overflow-hidden rounded-md border border-slate-800 bg-slate-900/60"
			data-testid="live-chart"
			bind:clientWidth={containerWidth}
		>
			{#if candlesQuery.isError}
				<div
					class="flex h-full min-h-[280px] flex-col items-center justify-center gap-3 p-6 text-center"
					data-testid="live-chart-error"
				>
					<p class="text-sm text-rose-300">
						Impossible de charger les bougies pour {symbol} ({exchange}).
					</p>
					<button
						type="button"
						data-testid="live-chart-retry"
						onclick={() => candlesQuery.refetch()}
						class="rounded-md border border-slate-600 bg-slate-800 px-3 py-1.5 text-sm text-slate-100 transition-colors hover:bg-slate-700"
					>
						Réessayer
					</button>
				</div>
			{:else if candlesQuery.isPending}
				<div
					class="flex h-full min-h-[280px] items-center justify-center p-6"
					data-testid="live-chart-loading"
					role="status"
					aria-live="polite"
				>
					<div
						class="h-6 w-6 animate-spin rounded-full border-2 border-slate-600 border-t-sky-400"
					></div>
					<span class="ml-3 text-sm text-slate-400">Chargement des bougies…</span>
				</div>
			{:else if candles.length === 0}
				<div
					class="flex h-full min-h-[280px] items-center justify-center p-6 text-center text-sm text-slate-400"
					data-testid="live-chart-empty"
				>
					Aucune bougie sur cette plage pour {symbol} ({exchange}).
				</div>
			{:else}
				<!-- Remount the chart on a new selection so the viewport refits to the new
		     series; within one selection it stays mounted (preserving zoom/pan for
		     the live polling to come). -->
				{#key candlesQueryKey(params).join('|')}
					<Chart
						{candles}
						{markers}
						subcharts={enabledSubcharts}
						onHover={handleHover}
						onSelect={selectBucket}
					/>
				{/key}
				{#if hovered}
					<div
						class="pointer-events-none absolute z-10"
						style="left: {tooltipLeft}px; top: {tooltipTop}px;"
					>
						<DecisionTooltip decision={hovered.decision} extra={hovered.extra} />
					</div>
				{/if}
				{#if selectedDecision}
					<DecisionPanel
						decision={selectedDecision}
						orders={panelOrders}
						fills={panelFills}
						fillsPending={fillsQuery.isPending}
						fillsError={fillsQuery.isError}
						position={selectedIndex + 1}
						total={selectedList.length}
						onPrev={() => stepPanel(-1)}
						onNext={() => stepPanel(1)}
						onClose={() => (selected = null)}
					/>
				{/if}
			{/if}
		</div>
		{#if candles.length > 0}
			<VolumeProfile {candles} />
		{/if}
	</div>
</div>

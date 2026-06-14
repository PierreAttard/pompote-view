<script lang="ts">
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import type { Candle, LiveDecision } from '$lib/api/types';
	import Chart, { type HoverInfo } from '$lib/chart/Chart.svelte';
	import IndicatorToggles from '$lib/chart/IndicatorToggles.svelte';
	import {
		buildDecisionLookup,
		COMPARE_MARKER_COLOR,
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
	import {
		boundedPollFrom,
		candlesQueryKey,
		fetchCandles,
		mergeCandles,
		POLL_INTERVAL_MS,
		type CandlesParams
	} from './candles';
	import DecisionPanel from './DecisionPanel.svelte';
	import DecisionTooltip from './DecisionTooltip.svelte';
	import { candlesToCsv, decisionsToCsv, downloadCanvasPng, downloadText } from './export';
	import FreshnessIndicator from './FreshnessIndicator.svelte';
	import { timeframeSeconds } from './depth';
	import { decisionsToIndicators } from './indicators';
	import VolumeProfile from './VolumeProfile.svelte';

	interface Props extends CandlesParams {
		/** Strategy whose orders/decisions annotate the chart. */
		strategyId: string;
		/** Optional second strategy to overlay for comparison (#34); '' = none. */
		strategyId2?: string;
	}

	// The selectors flow in as props; the page derives the `[from, to)` window.
	let { strategyId, strategyId2 = '', exchange, symbol, timeframe, from, to }: Props = $props();

	const params = $derived<CandlesParams>({ exchange, symbol, timeframe, from, to });
	const annotationParams = $derived<AnnotationParams>({ strategyId, from, to });
	const annotationParams2 = $derived<AnnotationParams>({ strategyId: strategyId2, from, to });

	const queryClient = useQueryClient();
	// Upper bound for live fetches: always "now", so each poll picks up candles /
	// orders created since the window opened. The query keys keep the original
	// (stable) `to`, so polling refetches in place without remounting the chart.
	const nowIso = () => new Date().toISOString();

	// Live polling (#26): refetch every 10s. The query key stays stable across
	// polls, so the chart is NOT remounted — it grows in place (zoom/pan kept).
	// TanStack pauses background refetches when the tab is hidden
	// (`refetchIntervalInBackground` defaults to false), satisfying the
	// "pause when hidden" requirement.
	const candlesQuery = createQuery(() => ({
		queryKey: candlesQueryKey(params),
		queryFn: async () => {
			// Incremental: fetch only `[lastTs, now]` and merge into the accumulated
			// candles, so each poll transfers just the tail (not the whole window).
			const prev = queryClient.getQueryData<Candle[]>(candlesQueryKey(params)) ?? [];
			const lastTs = prev.length > 0 ? prev[prev.length - 1].ts : params.from;
			// Bound the catch-up so a long-hidden tab can't overshoot the depth cap.
			const from = boundedPollFrom(lastTs, Date.now(), timeframeSeconds(timeframe) ?? 0);
			const fresh = await fetchCandles({ ...params, from, to: nowIso() });
			return mergeCandles(prev, fresh);
		},
		refetchInterval: POLL_INTERVAL_MS
	}));

	// Orders (buy/sell markers) and decisions (reason + snapshot) are best-effort
	// annotations: if they fail the chart still renders, just without markers. They
	// poll too (open `to`) so new markers appear in the same cycle; the payload is
	// small, so we refetch the window rather than merge incrementally.
	const ordersQuery = createQuery(() => ({
		queryKey: ordersQueryKey(annotationParams),
		queryFn: () => fetchOrders({ ...annotationParams, to: nowIso() }),
		enabled: strategyId !== '',
		refetchInterval: POLL_INTERVAL_MS
	}));
	const decisionsQuery = createQuery(() => ({
		queryKey: decisionsQueryKey(annotationParams),
		queryFn: () => fetchDecisions({ ...annotationParams, to: nowIso() }),
		enabled: strategyId !== '',
		refetchInterval: POLL_INTERVAL_MS
	}));

	// Second strategy to compare (#34): its orders become markers in a distinct
	// colour. Candles are NOT refetched per strategy, so polling is not doubled —
	// only this small orders query is added when a comparison is active.
	const ordersQuery2 = createQuery(() => ({
		queryKey: ordersQueryKey(annotationParams2),
		queryFn: () => fetchOrders({ ...annotationParams2, to: nowIso() }),
		enabled: strategyId2 !== '',
		refetchInterval: POLL_INTERVAL_MS
	}));

	const candles = $derived(candlesQuery.data ?? []);
	const orders = $derived(ordersQuery.data ?? []);
	const orders2 = $derived(ordersQuery2.data ?? []);
	const decisions = $derived(decisionsQuery.data ?? []);

	// Primary markers keep the buy-green / sell-red palette; the compared strategy
	// overlays its orders in violet (shape still encodes the side).
	const markers = $derived([
		...ordersToMarkers(orders),
		...ordersToMarkers(orders2, COMPARE_MARKER_COLOR)
	]);
	// Bucket-aligned lookup so a crosshair landing on a bar resolves to the
	// decisions whose orders fall in that candle.
	const lookup = $derived(buildDecisionLookup(orders, decisions, timeframeSeconds(timeframe) ?? 0));

	// Indicator sub-charts (#24): one toggleable line per numeric snapshot field,
	// extracted generically from the decisions. Off by default — the user enables
	// the ones they want; only enabled indicators get a pane (keeps it light).
	const indicators = $derived(decisionsToIndicators(decisions));
	let indicatorVisible = $state<Record<string, boolean>>({});
	const enabledSubcharts = $derived(indicators.filter((ind) => indicatorVisible[ind.id]));

	// Chart export (#33): the bound instance gives the PNG screenshot; the CSV is
	// built from the already-loaded candles/decisions of the current window.
	let chartComponent = $state<{ takeScreenshot(): HTMLCanvasElement | undefined }>();
	function exportPng(): void {
		const canvas = chartComponent?.takeScreenshot();
		if (canvas) downloadCanvasPng(canvas, `chart-${symbol}-${timeframe}.png`);
	}
	function exportCsv(): void {
		downloadText(`candles-${symbol}-${timeframe}.csv`, candlesToCsv(candles));
		if (decisions.length > 0) {
			downloadText(`decisions-${strategyId}.csv`, decisionsToCsv(decisions));
		}
	}

	// Decision detail panel (#22). A click selects a candle bucket; we keep the
	// bucket key + index (not the decision object) so the panel stays correct when
	// the underlying queries refetch (live polling, #26).
	let selected = $state<{ bucket: number; index: number } | null>(null);

	// Fills back the panel's executions table. Fetched lazily — only once a
	// decision is selected — then cached per window and reused across decisions.
	const fillsQuery = createQuery(() => ({
		queryKey: fillsQueryKey(annotationParams),
		queryFn: () => fetchFills({ ...annotationParams, to: nowIso() }),
		enabled: strategyId !== '' && selected !== null,
		refetchInterval: POLL_INTERVAL_MS
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
	{#if candles.length > 0}
		<!-- Export the current window (#33): PNG snapshot + CSV of candles/decisions. -->
		<div class="flex justify-end gap-2" data-testid="live-export">
			<button
				type="button"
				data-testid="export-png"
				onclick={exportPng}
				class="rounded-md border border-slate-700 bg-slate-800 px-2.5 py-1 text-xs text-slate-200 transition-colors hover:bg-slate-700"
			>
				Export PNG
			</button>
			<button
				type="button"
				data-testid="export-csv"
				onclick={exportCsv}
				class="rounded-md border border-slate-700 bg-slate-800 px-2.5 py-1 text-xs text-slate-200 transition-colors hover:bg-slate-700"
			>
				Export CSV
			</button>
		</div>
	{/if}
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
			data-candle-count={candles.length}
			bind:clientWidth={containerWidth}
		>
			<!-- Only surface the error state on the *initial* load (no data yet); a
			     transient poll failure must not replace a working chart (#26). -->
			{#if candlesQuery.isError && candles.length === 0}
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
						bind:this={chartComponent}
						{candles}
						{markers}
						subcharts={enabledSubcharts}
						onHover={handleHover}
						onSelect={selectBucket}
					/>
				{/key}
				<!-- Freshness badge (#27): its 1s ticker lives in the child, so it never
				     re-renders the chart. errorUpdatedAt > dataUpdatedAt ⇒ last poll failed. -->
				<div class="pointer-events-none absolute top-2 left-2 z-10">
					<FreshnessIndicator
						updatedAt={candlesQuery.dataUpdatedAt ?? 0}
						errored={(candlesQuery.errorUpdatedAt ?? 0) > (candlesQuery.dataUpdatedAt ?? 0)}
						fetching={candlesQuery.isFetching ?? false}
					/>
				</div>
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

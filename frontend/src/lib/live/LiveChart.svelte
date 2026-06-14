<script lang="ts">
	import { createQuery } from '@tanstack/svelte-query';
	import Chart from '$lib/chart/Chart.svelte';
	import { candlesQueryKey, fetchCandles, type CandlesParams } from './candles';

	// The selectors (strategy/exchange/symbol/timeframe/range) flow in as props;
	// the page derives the `[from, to)` window from the active preset.
	let { exchange, symbol, timeframe, from, to }: CandlesParams = $props();

	const params = $derived<CandlesParams>({ exchange, symbol, timeframe, from, to });

	// TanStack Query handles caching/dedup. The query key is derived from the
	// params, so changing any selector refetches and refreshes the chart. The 10s
	// live polling plugs in here in a later lot (refetchInterval); #18 stays
	// fetch-on-change only.
	const query = createQuery(() => ({
		queryKey: candlesQueryKey(params),
		queryFn: () => fetchCandles(params)
	}));

	const candles = $derived(query.data ?? []);
</script>

<div
	class="relative min-h-[280px] w-full overflow-hidden rounded-md border border-slate-800 bg-slate-900/60"
	data-testid="live-chart"
>
	{#if query.isError}
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
				onclick={() => query.refetch()}
				class="rounded-md border border-slate-600 bg-slate-800 px-3 py-1.5 text-sm text-slate-100 transition-colors hover:bg-slate-700"
			>
				Réessayer
			</button>
		</div>
	{:else if query.isPending}
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
			<Chart {candles} />
		{/key}
	{/if}
</div>

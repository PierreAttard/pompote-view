<script lang="ts">
	import { resolve } from '$app/paths';
	import Chart from '$lib/chart/Chart.svelte';
	import IndicatorToggles from '$lib/chart/IndicatorToggles.svelte';
	import BacktestRunSummaryPanel from '$lib/components/BacktestRunSummaryPanel.svelte';
	import BiasBanner from '$lib/components/BiasBanner.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	const run = $derived(data.detail.run);
	const degraded = $derived(data.source === 'none');
	const snapshot = $derived(JSON.stringify(data.detail.config_snapshot, null, 2));

	// Indicator overlays start hidden so the price action stays readable; the
	// user opts in per indicator. Visibility is keyed by overlay id.
	let visible = $state<Record<string, boolean>>({});
	const indicators = $derived(
		data.overlays.map((o) => ({ id: o.id, title: o.title, color: o.color }))
	);
	const shownOverlays = $derived(data.overlays.filter((o) => visible[o.id]));
</script>

<svelte:head>
	<title>{run.strategy_kind} · {run.symbol} — Backtest</title>
</svelte:head>

<section class="w-full max-w-5xl space-y-6">
	<header class="flex items-baseline justify-between">
		<h2 class="text-xl font-semibold text-slate-100">
			{run.strategy_kind} · {run.symbol}
		</h2>
		<a class="text-sm text-sky-300 hover:underline" href={resolve('/backtests')}>← Tous les runs</a>
	</header>

	<BiasBanner />

	<div class="rounded-md border border-slate-800 bg-slate-900/60 p-4">
		<BacktestRunSummaryPanel {run} />
	</div>

	<!-- Chart zone: OHLC background (Lot 4) + buy/sell markers (Lot 5) +
	     indicator overlays (Lot 6), with a clean timeline-only fallback when
	     `candles_5s` does not cover the run window (source = none). -->
	<div class="space-y-3" data-testid="chart-zone">
		{#if degraded}
			<div
				class="rounded-md border border-dashed border-slate-700 bg-slate-900/40 p-4 text-sm"
				data-testid="degraded-notice"
			>
				<p class="font-medium text-slate-300">Vue timeline (pas de fond de bougies)</p>
				<p class="mt-1 text-slate-400">
					La fenêtre de ce run n'est pas couverte par <code>candles_5s</code>. Le fond OHLC est
					masqué ; les ordres sont listés ci-dessous sur une timeline.
				</p>
			</div>

			{#if data.markers.length > 0}
				<ol class="space-y-1 text-sm" data-testid="timeline">
					{#each data.markers as marker (marker.ts + marker.side)}
						<li class="flex items-center gap-3 font-mono text-slate-300">
							<span class="text-slate-500">{marker.ts}</span>
							<span
								class={marker.side.toLowerCase() === 'buy' ? 'text-emerald-400' : 'text-rose-400'}
							>
								{marker.side.toUpperCase()}
							</span>
						</li>
					{/each}
				</ol>
			{:else}
				<p class="text-sm text-slate-500">Aucun ordre enregistré pour ce run.</p>
			{/if}
		{:else}
			{#if indicators.length > 0}
				<IndicatorToggles {indicators} bind:visible />
			{/if}
			<div class="h-[360px] overflow-hidden rounded-md border border-slate-800">
				<Chart candles={data.candles} markers={data.markers} overlays={shownOverlays} />
			</div>
			<p class="text-xs text-slate-500">
				{data.candleCount} bougies ({data.timeframe}) · {data.markers.length} ordres
			</p>
		{/if}
	</div>

	<details class="rounded-md border border-slate-800 bg-slate-900/60 p-4">
		<summary class="cursor-pointer text-sm text-slate-300">Configuration du run</summary>
		<pre
			class="mt-3 overflow-x-auto rounded bg-slate-950 p-3 text-xs text-slate-300"
			data-testid="config-snapshot">{snapshot}</pre>
	</details>
</section>

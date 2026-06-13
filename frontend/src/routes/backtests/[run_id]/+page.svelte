<script lang="ts">
	import { resolve } from '$app/paths';
	import BacktestRunSummaryPanel from '$lib/components/BacktestRunSummaryPanel.svelte';
	import BiasBanner from '$lib/components/BiasBanner.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	const run = $derived(data.detail.run);
	const degraded = $derived(data.source === 'none');
	const snapshot = $derived(JSON.stringify(data.detail.config_snapshot, null, 2));
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

	<!-- Chart zone. The OHLC chart (Lot 4), decision markers (Lot 5) and
	     indicator overlays (Lot 6) plug in here; this view ships the
	     source-aware container + the graceful degradation. -->
	<div
		class="flex min-h-[280px] items-center justify-center rounded-md border border-dashed border-slate-700 bg-slate-900/40 p-6 text-center text-sm"
		data-testid="chart-zone"
	>
		{#if degraded}
			<div data-testid="degraded-notice" class="max-w-md text-slate-400">
				<p class="font-medium text-slate-300">Vue timeline (pas de fond de bougies)</p>
				<p class="mt-1">
					La fenêtre de ce run n'est pas couverte par <code>candles_5s</code>. Les décisions et
					ordres s'afficheront sur une timeline (markers — Lot 5) dès que le chart sera en place.
				</p>
			</div>
		{:else}
			<div class="text-slate-400">
				<p class="font-medium text-slate-300">
					Fond OHLC disponible · {data.candleCount} bougies ({data.timeframe})
				</p>
				<p class="mt-1">Le chart Lightweight Charts (Lot 4) + markers (Lot 5) viennent ici.</p>
			</div>
		{/if}
	</div>

	<details class="rounded-md border border-slate-800 bg-slate-900/60 p-4">
		<summary class="cursor-pointer text-sm text-slate-300">Configuration du run</summary>
		<pre
			class="mt-3 overflow-x-auto rounded bg-slate-950 p-3 text-xs text-slate-300"
			data-testid="config-snapshot">{snapshot}</pre>
	</details>
</section>

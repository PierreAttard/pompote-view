<script lang="ts">
	import BacktestRunsTable from '$lib/components/BacktestRunsTable.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
</script>

<svelte:head>
	<title>Backtests — Pompote View</title>
</svelte:head>

<section class="w-full max-w-5xl">
	<header class="mb-4">
		<h2 class="text-xl font-semibold text-slate-100">Runs de backtest</h2>
		<p class="mt-1 text-sm text-slate-400">
			Sélectionne un run pour visualiser ses décisions et ses ordres rejoués.
		</p>
	</header>

	<!-- Selector filters: a plain GET form so filters live in the URL. -->
	<form method="GET" class="mb-6 flex flex-wrap items-end gap-3" data-testid="backtests-filters">
		<label class="flex flex-col text-xs text-slate-400">
			Statut
			<select
				name="status"
				aria-label="Filtrer par statut"
				class="mt-1 rounded border border-slate-700 bg-slate-900 px-2 py-1 text-sm text-slate-100"
			>
				<option value="" selected={!data.filters.status}>Tous</option>
				<option value="completed" selected={data.filters.status === 'completed'}>completed</option>
				<option value="running" selected={data.filters.status === 'running'}>running</option>
				<option value="failed" selected={data.filters.status === 'failed'}>failed</option>
				<option value="aborted" selected={data.filters.status === 'aborted'}>aborted</option>
			</select>
		</label>
		<label class="flex flex-col text-xs text-slate-400">
			Symbole
			<input
				name="symbol"
				aria-label="Filtrer par symbole"
				value={data.filters.symbol ?? ''}
				placeholder="BTCUSDT"
				class="mt-1 rounded border border-slate-700 bg-slate-900 px-2 py-1 text-sm text-slate-100"
			/>
		</label>
		<label class="flex flex-col text-xs text-slate-400">
			Exchange
			<input
				name="exchange"
				aria-label="Filtrer par exchange"
				value={data.filters.exchange ?? ''}
				placeholder="binance"
				class="mt-1 rounded border border-slate-700 bg-slate-900 px-2 py-1 text-sm text-slate-100"
			/>
		</label>
		<button
			type="submit"
			class="rounded bg-sky-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-sky-500"
		>
			Filtrer
		</button>
	</form>

	<BacktestRunsTable runs={data.runs} />
</section>

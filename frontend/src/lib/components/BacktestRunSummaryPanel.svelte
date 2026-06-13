<script lang="ts">
	import type { BacktestRunSummary } from '$lib/api/types';

	interface Props {
		run: BacktestRunSummary;
	}

	let { run }: Props = $props();

	const rows: { label: string; value: string }[] = $derived([
		{ label: 'Statut', value: run.status },
		{ label: 'Stratégie', value: run.strategy_kind },
		{ label: 'Exchange', value: run.exchange },
		{ label: 'Symbole', value: run.symbol },
		{ label: 'Fenêtre (market)', value: `${run.data_range_start} → ${run.data_range_end}` },
		{ label: 'Slippage', value: `${run.slippage_bps} bps` },
		{ label: 'Latence', value: `${run.latency_ms} ms` },
		{ label: 'Frais Binance', value: `${run.fee_binance_bps} bps` },
		{ label: 'Frais Kraken', value: `${run.fee_kraken_bps} bps` }
	]);
</script>

<dl class="grid grid-cols-2 gap-x-6 gap-y-2 text-sm sm:grid-cols-3" data-testid="run-summary">
	{#each rows as row (row.label)}
		<div>
			<dt class="text-xs text-slate-400">{row.label}</dt>
			<dd class="font-mono text-slate-100">{row.value}</dd>
		</div>
	{/each}
	{#if run.error}
		<div class="col-span-full">
			<dt class="text-xs text-rose-400">Erreur</dt>
			<dd class="font-mono text-rose-300">{run.error}</dd>
		</div>
	{/if}
</dl>

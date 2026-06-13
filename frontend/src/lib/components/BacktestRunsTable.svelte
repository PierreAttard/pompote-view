<script lang="ts">
	import { resolve } from '$app/paths';
	import type { BacktestRunSummary } from '$lib/api/types';

	interface Props {
		/** Runs to display, already ordered `started_at DESC` by the backend. */
		runs: BacktestRunSummary[];
	}

	let { runs }: Props = $props();

	/** Compact RFC3339 -> `YYYY-MM-DD HH:MM` (UTC) for dense table cells. */
	function fmtTs(ts: string): string {
		return ts.replace('T', ' ').slice(0, 16);
	}

	/** Tailwind classes per lifecycle status (chip colour). */
	function statusClass(status: string): string {
		switch (status) {
			case 'completed':
				return 'bg-emerald-500/15 text-emerald-300';
			case 'running':
				return 'bg-sky-500/15 text-sky-300';
			case 'failed':
				return 'bg-rose-500/15 text-rose-300';
			case 'aborted':
				return 'bg-amber-500/15 text-amber-300';
			default:
				return 'bg-slate-500/15 text-slate-300';
		}
	}
</script>

{#if runs.length === 0}
	<p data-testid="backtests-empty" class="text-sm text-slate-400">
		Aucun run de backtest pour ces filtres.
	</p>
{:else}
	<table class="w-full border-collapse text-left text-sm" data-testid="backtests-table">
		<thead>
			<tr class="border-b border-slate-800 text-xs text-slate-400">
				<th class="px-3 py-2 font-medium">Stratégie</th>
				<th class="px-3 py-2 font-medium">Symbole</th>
				<th class="px-3 py-2 font-medium">Exchange</th>
				<th class="px-3 py-2 font-medium">Fenêtre (market)</th>
				<th class="px-3 py-2 font-medium">Statut</th>
				<th class="px-3 py-2 font-medium">Démarré</th>
			</tr>
		</thead>
		<tbody>
			{#each runs as run (run.id)}
				<tr class="border-b border-slate-800/60 hover:bg-slate-800/40">
					<td class="px-3 py-2">
						<a
							class="text-sky-300 hover:underline"
							href={resolve('/backtests/[run_id]', { run_id: run.id })}
							data-testid="run-link"
						>
							{run.strategy_kind}
						</a>
					</td>
					<td class="px-3 py-2 text-slate-200">{run.symbol}</td>
					<td class="px-3 py-2 text-slate-300">{run.exchange}</td>
					<td class="px-3 py-2 font-mono text-xs text-slate-400">
						{fmtTs(run.data_range_start)} → {fmtTs(run.data_range_end)}
					</td>
					<td class="px-3 py-2">
						<span class="rounded px-2 py-0.5 text-xs {statusClass(run.status)}">
							{run.status}
						</span>
					</td>
					<td class="px-3 py-2 font-mono text-xs text-slate-400">{fmtTs(run.started_at)}</td>
				</tr>
			{/each}
		</tbody>
	</table>
{/if}

<script lang="ts">
	import type { LiveDecision } from '$lib/api/types';
	import { flattenSnapshot } from './snapshot';

	interface Props {
		/** The decision whose rationale + snapshot to display. */
		decision: LiveDecision;
	}

	let { decision }: Props = $props();

	// The market-context snapshot is opaque JSONB; flatten it generically.
	const rows = $derived(flattenSnapshot(decision.market_context));
</script>

<div
	class="pointer-events-none w-64 max-w-xs rounded-md border border-slate-700 bg-slate-900/95 p-3 text-xs shadow-lg"
	data-testid="decision-tooltip"
	role="tooltip"
>
	<p class="font-semibold text-slate-200">Décision</p>
	<p class="mt-1 text-slate-300" data-testid="decision-reason">
		{decision.reason || '—'}
	</p>

	{#if rows.length > 0}
		<dl class="mt-2 grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5 border-t border-slate-800 pt-2">
			{#each rows as row (row.label)}
				<dt class="truncate font-mono text-slate-400" title={row.label}>{row.label}</dt>
				<dd class="text-right font-mono text-slate-200">{row.value}</dd>
			{/each}
		</dl>
	{:else}
		<p class="mt-2 border-t border-slate-800 pt-2 text-slate-500">Pas de snapshot d'indicateurs.</p>
	{/if}
</div>

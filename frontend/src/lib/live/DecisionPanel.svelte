<script lang="ts">
	import type { LiveDecision, LiveFill, Order } from '$lib/api/types';
	import { formatNumber, formatTimestamp } from './format';
	import { flattenSnapshot } from './snapshot';

	interface Props {
		/** The decision whose full context to display. */
		decision: LiveDecision;
		/** Orders emitted by this decision (already joined on `decision_id`). */
		orders: Order[];
		/** Fills of those orders (already joined on `order_id`). */
		fills: LiveFill[];
		/** Fills query still in flight (shows a loading hint). */
		fillsPending?: boolean;
		/** Fills query failed (shows an "unavailable" hint — best-effort). */
		fillsError?: boolean;
		/** 1-based position within a same-candle group of decisions (for the stepper). */
		position?: number;
		/** Size of the same-candle group; a stepper shows only when `> 1`. */
		total?: number;
		/** Step to the previous decision in the group. */
		onPrev?: () => void;
		/** Step to the next decision in the group. */
		onNext?: () => void;
		/** Close the panel. */
		onClose: () => void;
	}

	let {
		decision,
		orders,
		fills,
		fillsPending = false,
		fillsError = false,
		position = 1,
		total = 1,
		onPrev,
		onNext,
		onClose
	}: Props = $props();

	// The market-context snapshot is opaque JSONB; flatten it generically, and
	// also offer the raw JSON as a debug view.
	const rows = $derived(flattenSnapshot(decision.market_context));
	const rawJson = $derived(JSON.stringify(decision.market_context ?? {}, null, 2));

	function sideClass(side: string): string {
		const s = String(side ?? '').toLowerCase();
		return s === 'buy' ? 'text-emerald-400' : s === 'sell' ? 'text-rose-400' : 'text-slate-300';
	}

	// Move focus into the panel on open so keyboard users land in the dialog, and
	// restore it to the previously focused element (e.g. the chart) on close.
	let closeButton = $state<HTMLButtonElement>();
	$effect(() => {
		const previouslyFocused = document.activeElement as HTMLElement | null;
		closeButton?.focus();
		return () => previouslyFocused?.focus?.();
	});
</script>

<svelte:window
	onkeydown={(e) => {
		if (e.key === 'Escape') {
			e.preventDefault();
			onClose();
		}
	}}
/>

<!-- Generic element (not <aside>) so the interactive `dialog` role is valid; the
     panel is non-modal and dismissible via ✕ / Escape, with focus moved inside. -->
<div
	class="absolute inset-y-0 right-0 z-20 flex w-80 max-w-full flex-col overflow-y-auto border-l border-slate-700 bg-slate-900/95 text-xs shadow-xl backdrop-blur-sm"
	data-testid="decision-panel"
	role="dialog"
	aria-modal="false"
	aria-labelledby="decision-panel-title"
>
	<header
		class="sticky top-0 flex items-center justify-between gap-2 border-b border-slate-800 bg-slate-900/95 px-3 py-2"
	>
		<h3 id="decision-panel-title" class="text-sm font-semibold text-slate-100">
			Détail de la décision
		</h3>
		<div class="flex items-center gap-1">
			{#if total > 1}
				<div class="flex items-center gap-1" data-testid="panel-stepper">
					<button
						type="button"
						data-testid="panel-prev"
						aria-label="Décision précédente"
						onclick={() => onPrev?.()}
						class="rounded border border-slate-700 px-1.5 py-0.5 text-slate-300 transition-colors hover:bg-slate-800"
					>
						‹
					</button>
					<span class="font-mono text-slate-400" data-testid="panel-position"
						>{position}/{total}</span
					>
					<button
						type="button"
						data-testid="panel-next"
						aria-label="Décision suivante"
						onclick={() => onNext?.()}
						class="rounded border border-slate-700 px-1.5 py-0.5 text-slate-300 transition-colors hover:bg-slate-800"
					>
						›
					</button>
				</div>
			{/if}
			<button
				type="button"
				bind:this={closeButton}
				data-testid="decision-panel-close"
				aria-label="Fermer le panneau"
				onclick={() => onClose()}
				class="rounded border border-slate-700 px-2 py-0.5 text-slate-300 transition-colors hover:bg-slate-800"
			>
				✕
			</button>
		</div>
	</header>

	<div class="space-y-3 p-3">
		<!-- Reason (extended) -->
		<section>
			<p class="font-semibold text-slate-300">Motif</p>
			<p class="mt-1 text-slate-200" data-testid="panel-reason">{decision.reason || '—'}</p>
		</section>

		<!-- Decision metadata -->
		<section>
			<dl class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5 border-t border-slate-800 pt-2">
				<dt class="text-slate-400">Horodatage</dt>
				<dd class="text-right font-mono text-slate-200">
					{formatTimestamp(decision.created_at)} UTC
				</dd>
				<dt class="text-slate-400">Ordres</dt>
				<dd class="text-right font-mono text-slate-200">{decision.orders_count}</dd>
				<dt class="text-slate-400">Session</dt>
				<dd class="truncate text-right font-mono text-slate-400" title={decision.session_id}>
					{decision.session_id}
				</dd>
				<dt class="text-slate-400">Décision</dt>
				<dd class="truncate text-right font-mono text-slate-400" title={decision.decision_id}>
					{decision.decision_id}
				</dd>
			</dl>
		</section>

		<!-- Indicator snapshot (flattened) -->
		<section>
			<p class="font-semibold text-slate-300">Snapshot d'indicateurs</p>
			{#if rows.length > 0}
				<dl
					class="mt-1 grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5 border-t border-slate-800 pt-2"
					data-testid="panel-snapshot"
				>
					{#each rows as row (row.label)}
						<dt class="truncate font-mono text-slate-400" title={row.label}>{row.label}</dt>
						<dd class="text-right font-mono text-slate-200">{row.value}</dd>
					{/each}
				</dl>
			{:else}
				<p class="mt-1 border-t border-slate-800 pt-2 text-slate-500">
					Pas de snapshot d'indicateurs.
				</p>
			{/if}
			<!-- Raw JSON debug view, collapsed by default to stay compact. -->
			<details class="mt-2">
				<summary class="cursor-pointer text-slate-400 hover:text-slate-200">JSON brut</summary>
				<pre
					class="mt-1 max-h-48 overflow-auto rounded bg-slate-950/70 p-2 font-mono text-[11px] leading-snug text-slate-300"
					data-testid="panel-snapshot-json">{rawJson}</pre>
			</details>
		</section>

		<!-- Associated orders -->
		<section data-testid="panel-orders">
			<p class="font-semibold text-slate-300">Ordres ({orders.length})</p>
			{#if orders.length > 0}
				<table class="mt-1 w-full border-t border-slate-800 pt-2 text-left">
					<thead class="text-slate-500">
						<tr>
							<th class="font-medium">Sens</th>
							<th class="font-medium">Statut</th>
							<th class="text-right font-medium">Qté</th>
							<th class="text-right font-medium">Prix</th>
						</tr>
					</thead>
					<tbody class="font-mono text-slate-300">
						{#each orders as o (o.order_id)}
							<tr class="border-t border-slate-800/60">
								<td class={sideClass(o.side)}>{o.side}</td>
								<td class="text-slate-400">{o.status}</td>
								<td class="text-right">{formatNumber(o.quantity)}</td>
								<td class="text-right">{formatNumber(o.price)}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			{:else}
				<p class="mt-1 border-t border-slate-800 pt-2 text-slate-500">Aucun ordre.</p>
			{/if}
		</section>

		<!-- Associated fills (best-effort) -->
		<section data-testid="panel-fills">
			<p class="font-semibold text-slate-300">Exécutions</p>
			{#if fillsPending}
				<p class="mt-1 border-t border-slate-800 pt-2 text-slate-500">Chargement des exécutions…</p>
			{:else if fillsError}
				<p class="mt-1 border-t border-slate-800 pt-2 text-slate-500">Exécutions indisponibles.</p>
			{:else if fills.length > 0}
				<table class="mt-1 w-full border-t border-slate-800 pt-2 text-left">
					<thead class="text-slate-500">
						<tr>
							<th class="font-medium">Sens</th>
							<th class="text-right font-medium">Prix</th>
							<th class="text-right font-medium">Qté</th>
							<th class="text-right font-medium">Frais</th>
							<th class="text-right font-medium">Mode</th>
						</tr>
					</thead>
					<tbody class="font-mono text-slate-300">
						{#each fills as f (f.fill_id)}
							<tr class="border-t border-slate-800/60">
								<td class={sideClass(f.side)}>{f.side}</td>
								<td class="text-right">{formatNumber(f.price)}</td>
								<td class="text-right">{formatNumber(f.quantity)}</td>
								<td class="text-right">{formatNumber(f.fee)} {f.fee_asset}</td>
								<td class="text-right">
									<span
										class="rounded px-1 py-0.5 text-[10px] {f.is_paper
											? 'bg-amber-900/40 text-amber-300'
											: 'bg-sky-900/40 text-sky-300'}"
									>
										{f.is_paper ? 'paper' : 'live'}
									</span>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			{:else}
				<p class="mt-1 border-t border-slate-800 pt-2 text-slate-500">Aucune exécution.</p>
			{/if}
		</section>
	</div>
</div>

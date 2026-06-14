<script lang="ts">
	import { formatAge } from './format';

	interface Props {
		/** Epoch ms of the last successful update (`dataUpdatedAt`); 0 = never. */
		updatedAt: number;
		/** The most recent poll attempt failed (`errorUpdatedAt > dataUpdatedAt`). */
		errored?: boolean;
		/** A poll is currently in flight — pulses the dot. */
		fetching?: boolean;
		/** Age beyond which the indicator turns stale (default 30s, per #27). */
		staleAfterMs?: number;
	}

	let { updatedAt, errored = false, fetching = false, staleAfterMs = 30_000 }: Props = $props();

	// Local 1s ticker, kept in this component so the timer re-evaluates only this
	// badge — never the chart (issue #27: "no costly chart re-render").
	let now = $state(Date.now());
	$effect(() => {
		const id = setInterval(() => (now = Date.now()), 1000);
		return () => clearInterval(id);
	});

	const age = $derived(Math.max(0, now - updatedAt));
	const level = $derived(errored ? 'error' : age > staleAfterMs ? 'stale' : 'fresh');
	const ageLabel = $derived(formatAge(age));
	const dotClass = $derived(
		level === 'error' ? 'bg-rose-500' : level === 'stale' ? 'bg-amber-400' : 'bg-emerald-400'
	);
	const levelLabel = $derived(
		level === 'error' ? 'Erreur de mise à jour' : level === 'stale' ? 'En retard' : 'À jour'
	);
	const ariaLabel = $derived(`${levelLabel} — dernière mise à jour ${ageLabel}`);
</script>

{#if updatedAt > 0}
	<div
		class="pointer-events-none flex items-center gap-1.5 rounded-full border border-slate-700 bg-slate-900/80 px-2 py-0.5 text-[11px] text-slate-300 shadow-sm"
		data-testid="freshness-indicator"
		data-state={level}
		role="img"
		aria-label={ariaLabel}
	>
		<span class="inline-block h-2 w-2 rounded-full {dotClass} {fetching ? 'animate-pulse' : ''}"
		></span>
		<span>{ageLabel}</span>
	</div>
{/if}

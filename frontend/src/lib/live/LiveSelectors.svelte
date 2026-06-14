<script lang="ts">
	import type { Strategy } from '$lib/api/types';
	import { RANGE_PRESETS, type RangePreset } from './range';

	/** Trading-mode filter applied to the strategy list. */
	export type ModeFilter = 'all' | 'paper' | 'live';

	interface Props {
		strategies: Strategy[];
		timeframes: string[];
		mode: ModeFilter;
		strategyId: string;
		symbol: string;
		timeframe: string;
		preset: RangePreset;
		/** Called with the changed query-param key and its new value (or null to clear). */
		onChange: (key: string, value: string | null) => void;
	}

	let { strategies, timeframes, mode, strategyId, symbol, timeframe, preset, onChange }: Props =
		$props();

	// Strategies are filtered by the paper/live mode so the user only picks a
	// strategy actually enabled for the mode they are monitoring.
	const visibleStrategies = $derived(
		strategies.filter((s) =>
			mode === 'paper' ? s.enabled_paper : mode === 'live' ? s.enabled_live : true
		)
	);
</script>

<form
	class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3"
	data-testid="live-selectors"
	onsubmit={(e) => e.preventDefault()}
>
	<label class="flex flex-col gap-1 text-sm">
		<span class="text-xs text-slate-400">Mode</span>
		<select
			class="rounded-md border border-slate-700 bg-slate-900 px-2 py-1.5 text-slate-100"
			value={mode}
			data-testid="select-mode"
			onchange={(e) => onChange('mode', e.currentTarget.value)}
		>
			<option value="all">Tous</option>
			<option value="paper">Paper</option>
			<option value="live">Live</option>
		</select>
	</label>

	<label class="flex flex-col gap-1 text-sm">
		<span class="text-xs text-slate-400">Stratégie</span>
		<select
			class="rounded-md border border-slate-700 bg-slate-900 px-2 py-1.5 text-slate-100"
			value={strategyId}
			data-testid="select-strategy"
			onchange={(e) => onChange('strategy', e.currentTarget.value || null)}
		>
			<option value="">— Choisir —</option>
			{#each visibleStrategies as s (s.id)}
				<option value={s.id}>{s.name} ({s.kind})</option>
			{/each}
		</select>
	</label>

	<label class="flex flex-col gap-1 text-sm">
		<span class="text-xs text-slate-400">Symbole</span>
		<input
			class="rounded-md border border-slate-700 bg-slate-900 px-2 py-1.5 font-mono text-slate-100"
			value={symbol}
			placeholder="BTCUSDT"
			data-testid="input-symbol"
			onchange={(e) => onChange('symbol', e.currentTarget.value || null)}
		/>
	</label>

	<label class="flex flex-col gap-1 text-sm">
		<span class="text-xs text-slate-400">Timeframe</span>
		<select
			class="rounded-md border border-slate-700 bg-slate-900 px-2 py-1.5 text-slate-100"
			value={timeframe}
			data-testid="select-timeframe"
			onchange={(e) => onChange('timeframe', e.currentTarget.value)}
		>
			{#each timeframes as tf (tf)}
				<option value={tf}>{tf}</option>
			{/each}
		</select>
	</label>

	<fieldset class="flex flex-col gap-1 text-sm sm:col-span-2">
		<span class="text-xs text-slate-400">Plage</span>
		<div class="flex flex-wrap gap-2" role="group" aria-label="Plage temporelle">
			{#each RANGE_PRESETS as p (p.id)}
				<button
					type="button"
					aria-pressed={preset === p.id}
					data-testid={`preset-${p.id}`}
					onclick={() => onChange('preset', p.id)}
					class="rounded-full border px-3 py-1 text-xs transition-colors {preset === p.id
						? 'border-sky-500 bg-sky-900/40 text-sky-200'
						: 'border-slate-700 bg-slate-900 text-slate-300'}"
				>
					{p.label}
				</button>
			{/each}
		</div>
	</fieldset>
</form>

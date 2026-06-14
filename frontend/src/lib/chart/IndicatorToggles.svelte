<script lang="ts">
	/**
	 * Reusable on/off selector for indicator overlays. Source-agnostic: the
	 * consumer owns the `overlays` data and filters it by the bindable `visible`
	 * map before handing it to `<Chart>`. Distinguishes indicators by a colour
	 * swatch *and* the label/`aria-pressed` state, not colour alone (a11y).
	 */
	interface Indicator {
		id: string;
		title?: string;
		color: string;
	}

	interface Props {
		indicators: Indicator[];
		/** Map of indicator id → shown. Two-way bound so toggles update it. */
		visible: Record<string, boolean>;
	}

	let { indicators, visible = $bindable() }: Props = $props();

	function toggle(id: string): void {
		visible = { ...visible, [id]: !visible[id] };
	}
</script>

<div
	class="flex flex-wrap gap-2"
	role="group"
	aria-label="Indicateurs"
	data-testid="indicator-toggles"
>
	{#each indicators as indicator (indicator.id)}
		{@const on = visible[indicator.id] ?? false}
		<button
			type="button"
			aria-pressed={on}
			onclick={() => toggle(indicator.id)}
			class="inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs font-medium transition-colors {on
				? 'border-slate-500 bg-slate-700 text-slate-100'
				: 'border-slate-700 bg-slate-900 text-slate-400'}"
		>
			<span
				class="inline-block h-2 w-2 rounded-full"
				style="background-color: {indicator.color}"
				aria-hidden="true"
			></span>
			{indicator.title ?? indicator.id}
		</button>
	{/each}
</div>

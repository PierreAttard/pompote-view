<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import LiveSelectors, { type ModeFilter } from '$lib/live/LiveSelectors.svelte';
	import { isRangePreset, presetWindow, type RangePreset } from '$lib/live/range';
	import { nextQuery } from '$lib/live/url';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	// Selection lives in the URL (shareable/bookmarkable). The controls read
	// these and write back via `update()`.
	const params = $derived(page.url.searchParams);
	const mode = $derived((params.get('mode') ?? 'all') as ModeFilter);
	const strategyId = $derived(params.get('strategy') ?? '');
	const symbol = $derived(params.get('symbol') ?? '');
	const timeframe = $derived(params.get('timeframe') ?? data.timeframes[0] ?? '1h');
	const preset = $derived<RangePreset>(
		isRangePreset(params.get('preset')) ? (params.get('preset') as RangePreset) : '24h'
	);

	const window = $derived(presetWindow(preset));
	const selectedStrategy = $derived(data.strategies.find((s) => s.id === strategyId) ?? null);
	const ready = $derived(strategyId !== '' && symbol !== '');

	function update(key: string, value: string | null): void {
		const changes: Record<string, string | null> = { [key]: value };
		// Switching mode can orphan the selected strategy (not enabled for the new
		// mode): drop it from the URL so the dropdown and selection stay coherent.
		if (key === 'mode') {
			const sel = data.strategies.find((s) => s.id === strategyId);
			const stillValid =
				!sel ||
				value === 'all' ||
				value === null ||
				(value === 'paper' ? sel.enabled_paper : sel.enabled_live);
			if (!stillValid) changes.strategy = null;
		}
		// Resolve the route so navigation stays base-path-safe; only a same-origin
		// query string is appended.
		const qs = nextQuery(page.url.searchParams, changes);
		const target = qs ? `${resolve('/live')}?${qs}` : resolve('/live');
		// Path is already resolved via `resolve('/live')`; only a same-origin query
		// string is appended, so the navigation is base-path-safe.
		// eslint-disable-next-line svelte/no-navigation-without-resolve
		void goto(target, { replaceState: true, keepFocus: true, noScroll: true });
	}
</script>

<svelte:head>
	<title>Live — Pompote View</title>
</svelte:head>

<section class="w-full max-w-5xl space-y-6">
	<header>
		<h2 class="text-xl font-semibold text-slate-100">Monitoring live</h2>
		<p class="mt-1 text-sm text-slate-400">
			Sélectionne une stratégie, un symbole, un timeframe et une plage. Paper et live sont
			distingués par le filtre de mode.
		</p>
	</header>

	<div class="rounded-md border border-slate-800 bg-slate-900/60 p-4">
		<LiveSelectors
			strategies={data.strategies}
			timeframes={data.timeframes}
			{mode}
			{strategyId}
			{symbol}
			{timeframe}
			{preset}
			onChange={update}
		/>
	</div>

	<!-- Chart zone. The OHLC chart (reusing $lib/chart) + live markers/overlays
	     + 10s polling plug in here from #18 onwards. -->
	<div
		class="flex min-h-[280px] items-center justify-center rounded-md border border-dashed border-slate-700 bg-slate-900/40 p-6 text-center text-sm"
		data-testid="live-chart-placeholder"
	>
		{#if ready}
			<div class="text-slate-400">
				<p class="font-medium text-slate-300">
					{selectedStrategy?.name ?? strategyId} · {symbol} · {timeframe}
				</p>
				<p class="mt-1 font-mono text-xs">{window.from} → {window.to}</p>
				<p class="mt-2">Le chart live (candles + markers + polling 10s) arrive en #18.</p>
			</div>
		{:else}
			<p class="max-w-md text-slate-400">
				Choisis une <strong>stratégie</strong> et saisis un <strong>symbole</strong> pour afficher le
				graphique.
			</p>
		{/if}
	</div>
</section>

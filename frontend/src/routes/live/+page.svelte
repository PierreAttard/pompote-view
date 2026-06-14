<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { depthStatus } from '$lib/live/depth';
	import { DEFAULT_EXCHANGE, isExchange } from '$lib/live/exchanges';
	import LiveChart from '$lib/live/LiveChart.svelte';
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
	const exchange = $derived(
		isExchange(params.get('exchange')) ? params.get('exchange')! : DEFAULT_EXCHANGE
	);
	const symbol = $derived(params.get('symbol') ?? '');
	const timeframe = $derived(params.get('timeframe') ?? data.timeframes[0] ?? '1h');
	const preset = $derived<RangePreset>(
		isRangePreset(params.get('preset')) ? (params.get('preset') as RangePreset) : '24h'
	);

	const window = $derived(presetWindow(preset));
	const selectedStrategy = $derived(data.strategies.find((s) => s.id === strategyId) ?? null);
	const ready = $derived(strategyId !== '' && symbol !== '');

	// Guard the backend's 5000-point cap on the UI: when the (timeframe, range)
	// pair would overflow, we refuse to fire the candles query (it would 400
	// anyway) and suggest the finest timeframe that fits.
	const depth = $derived(depthStatus(preset, timeframe, data.timeframes));

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
			{exchange}
			{symbol}
			{timeframe}
			{preset}
			onChange={update}
		/>
	</div>

	<!-- Chart zone. Decision markers/overlays + 10s polling plug into LiveChart
	     in later lots; #18 wires the candles fetch + native zoom/pan. -->
	{#if ready && depth.exceeds}
		<div
			class="space-y-2 rounded-md border border-amber-700/60 bg-amber-950/30 p-4 text-sm"
			data-testid="live-depth-warning"
			role="alert"
		>
			<p class="text-amber-200">
				Plage trop large pour ce timeframe. Réduisez la plage ou augmentez le timeframe.
			</p>
			<p class="text-xs text-amber-300/80">
				~{depth.count?.toLocaleString('fr-FR')} points demandés, au-delà du plafond de {(5000).toLocaleString(
					'fr-FR'
				)}.
			</p>
			{#if depth.suggestion}
				<button
					type="button"
					data-testid="live-depth-suggestion"
					onclick={() => update('timeframe', depth.suggestion)}
					class="rounded-md border border-amber-600 bg-amber-900/40 px-3 py-1.5 text-amber-100 transition-colors hover:bg-amber-800/50"
				>
					Passer en {depth.suggestion}
				</button>
			{/if}
		</div>
	{:else if ready}
		<div data-testid="live-chart-header" class="text-sm text-slate-300">
			<span class="font-medium">{selectedStrategy?.name ?? strategyId}</span>
			<span class="text-slate-500"> · {exchange} · {symbol} · {timeframe}</span>
			<span class="ml-1 font-mono text-xs text-slate-500">{window.from} → {window.to}</span>
		</div>
		<LiveChart {exchange} {symbol} {timeframe} from={window.from} to={window.to} />
	{:else}
		<div
			class="flex min-h-[280px] items-center justify-center rounded-md border border-dashed border-slate-700 bg-slate-900/40 p-6 text-center text-sm"
			data-testid="live-chart-placeholder"
		>
			<p class="max-w-md text-slate-400">
				Choisis une <strong>stratégie</strong> et saisis un <strong>symbole</strong> pour afficher le
				graphique.
			</p>
		</div>
	{/if}
</section>

<script lang="ts">
	import type { Candle } from '$lib/api/types';
	import { formatNumber } from './format';
	import { computeVolumeProfile } from './volumeProfile';

	interface Props {
		/** Candles to aggregate volume-by-price from (same data as the chart). */
		candles: Candle[];
		/** Number of price levels (default 24). */
		bucketCount?: number;
	}

	let { candles, bucketCount = 24 }: Props = $props();

	const profile = $derived(computeVolumeProfile(candles, bucketCount));
	// Highest price at the top, like a price axis.
	const rows = $derived(profile.buckets.map((bucket, i) => ({ bucket, i })).reverse());

	function barClass(i: number): string {
		if (i === profile.pocIndex) return 'bg-amber-500';
		if (profile.hvnIndices.includes(i)) return 'bg-emerald-500';
		if (profile.lvnIndices.includes(i)) return 'bg-slate-700';
		return 'bg-slate-500';
	}

	const inValueArea = (i: number) => i >= profile.valueAreaLow && i <= profile.valueAreaHigh;
	const width = (volume: number) =>
		profile.maxVolume > 0 ? (volume / profile.maxVolume) * 100 : 0;

	const pocPrice = $derived(profile.pocIndex >= 0 ? profile.buckets[profile.pocIndex].price : null);
	const vaLowPrice = $derived(
		profile.valueAreaLow >= 0 ? profile.buckets[profile.valueAreaLow].low : null
	);
	const vaHighPrice = $derived(
		profile.valueAreaHigh >= 0 ? profile.buckets[profile.valueAreaHigh].high : null
	);
</script>

<aside
	class="flex min-h-[280px] w-44 max-w-[45%] flex-col overflow-hidden rounded-md border border-slate-800 bg-slate-900/60 text-xs"
	data-testid="volume-profile"
	role="img"
	aria-label={pocPrice !== null
		? `Volume profile. POC ${formatNumber(pocPrice)}, value area ${formatNumber(vaLowPrice)} à ${formatNumber(vaHighPrice)}.`
		: 'Volume profile (aucune donnée).'}
>
	<header class="border-b border-slate-800 px-2 py-1 font-semibold text-slate-300">
		Volume profile
	</header>

	{#if rows.length === 0}
		<div class="flex flex-1 items-center justify-center p-2 text-center text-slate-500">
			Pas de volume sur cette plage.
		</div>
	{:else}
		<div class="flex flex-1 flex-col">
			{#each rows as { bucket, i } (i)}
				<div
					class="relative flex-1 {inValueArea(i) ? 'bg-sky-500/10' : ''}"
					title="{formatNumber(bucket.price)} · vol {formatNumber(bucket.volume)}"
					data-testid="vp-bucket"
				>
					<div
						class="absolute inset-y-px left-0 rounded-r-sm {barClass(i)}"
						style="width: {width(bucket.volume)}%"
					></div>
					{#if i === profile.pocIndex}
						<span
							class="absolute inset-y-0 right-1 flex items-center font-mono text-[10px] text-amber-200"
							data-testid="vp-poc"
						>
							POC {formatNumber(bucket.price)}
						</span>
					{/if}
				</div>
			{/each}
		</div>

		<footer class="border-t border-slate-800 px-2 py-1 text-slate-400" data-testid="vp-value-area">
			VA {formatNumber(vaLowPrice)} – {formatNumber(vaHighPrice)}
		</footer>
	{/if}
</aside>

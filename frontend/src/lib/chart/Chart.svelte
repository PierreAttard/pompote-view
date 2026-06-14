<script lang="ts">
	import { onMount } from 'svelte';
	import {
		createChart,
		CandlestickSeries,
		ColorType,
		type CandlestickData,
		type IChartApi,
		type ISeriesApi,
		type UTCTimestamp
	} from 'lightweight-charts';
	import type { Candle } from '$lib/api/types';

	interface Props {
		/**
		 * OHLC candles to render. The component is source-agnostic: live and
		 * backtest views both feed it the same shape (decision markers and
		 * indicator overlays land here in later lots).
		 */
		candles?: Candle[];
		/** Override the system colour-scheme preference (mostly for tests). */
		dark?: boolean;
	}

	let { candles = [], dark }: Props = $props();

	let container: HTMLDivElement;
	let chart: IChartApi | undefined;
	let series: ISeriesApi<'Candlestick'> | undefined;

	/**
	 * Lightweight Charts wants strictly ascending, de-duplicated timestamps in
	 * seconds. The API returns RFC3339 strings on `ts`, so we parse, sort, and
	 * drop duplicate buckets (keeping the last one) before handing data over.
	 */
	function toSeriesData(input: Candle[]): CandlestickData[] {
		const points: CandlestickData[] = [];
		for (const c of input) {
			const time = Math.floor(Date.parse(c.ts) / 1000);
			if (Number.isNaN(time)) continue;
			points.push({ time: time as UTCTimestamp, open: c.o, high: c.h, low: c.l, close: c.c });
		}
		points.sort((a, b) => (a.time as number) - (b.time as number));
		// Drop duplicate buckets, keeping the last occurrence for each timestamp.
		return points.filter((p, i) => i === points.length - 1 || p.time !== points[i + 1].time);
	}

	function isDark(): boolean {
		if (dark !== undefined) return dark;
		return window.matchMedia?.('(prefers-color-scheme: dark)').matches ?? true;
	}

	onMount(() => {
		const palette = isDark()
			? { background: '#0f172a', text: '#cbd5e1', grid: '#1e293b' }
			: { background: '#ffffff', text: '#334155', grid: '#e2e8f0' };

		chart = createChart(container, {
			autoSize: true,
			layout: {
				background: { type: ColorType.Solid, color: palette.background },
				textColor: palette.text
			},
			grid: {
				vertLines: { color: palette.grid },
				horzLines: { color: palette.grid }
			},
			timeScale: { timeVisible: true, secondsVisible: false }
		});

		series = chart.addSeries(CandlestickSeries);
		series.setData(toSeriesData(candles));
		chart.timeScale().fitContent();

		// Disposing the chart tears down its ResizeObserver and DOM nodes, so
		// repeated mount/unmount cycles do not leak.
		return () => {
			chart?.remove();
			chart = undefined;
			series = undefined;
		};
	});

	// React to candle changes after the initial mount.
	$effect(() => {
		const data = toSeriesData(candles);
		if (series) {
			series.setData(data);
			chart?.timeScale().fitContent();
		}
	});
</script>

<div bind:this={container} data-testid="chart" class="h-full min-h-[280px] w-full"></div>

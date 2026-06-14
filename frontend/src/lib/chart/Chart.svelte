<script lang="ts">
	import { onMount } from 'svelte';
	import {
		createChart,
		createSeriesMarkers,
		CandlestickSeries,
		ColorType,
		type IChartApi,
		type ISeriesApi,
		type ISeriesMarkersPluginApi,
		type Time
	} from 'lightweight-charts';
	import type { Candle } from '$lib/api/types';
	import { toSeriesData } from './candles';
	import { toSeriesMarkers, type ChartMarker } from './markers';

	interface Props {
		/**
		 * OHLC candles to render. The component is source-agnostic: live and
		 * backtest views both feed it the same shape (decision markers and
		 * indicator overlays land here in later lots).
		 */
		candles?: Candle[];
		/** Buy/sell annotations placed at each decision/order timestamp. */
		markers?: ChartMarker[];
		/** Override the system colour-scheme preference (mostly for tests). */
		dark?: boolean;
	}

	let { candles = [], markers = [], dark }: Props = $props();

	let container: HTMLDivElement;
	let chart: IChartApi | undefined;
	let series: ISeriesApi<'Candlestick'> | undefined;
	let markersPlugin: ISeriesMarkersPluginApi<Time> | undefined;
	// Fit the viewport only on the first load; later updates (e.g. live polling
	// in #18) must not clobber the user's zoom/pan.
	let fitted = false;

	function isDark(): boolean {
		if (dark !== undefined) return dark;
		return window.matchMedia?.('(prefers-color-scheme: dark)').matches ?? true;
	}

	/**
	 * Push candles into the series. Called from both `onMount` (initial load,
	 * guaranteed to have the series) and the `$effect` (later changes); the
	 * extra call is idempotent, which keeps us correct regardless of the
	 * mount/effect ordering. The viewport is fitted only once.
	 */
	function render(input: Candle[]): void {
		if (!series) return;
		const data = toSeriesData(input);
		series.setData(data);
		if (!fitted && data.length > 0) {
			chart?.timeScale().fitContent();
			fitted = true;
		}
	}

	/** Replace the buy/sell markers (no-op until the series exists). */
	function renderMarkers(input: ChartMarker[]): void {
		if (!series) return;
		const data = toSeriesMarkers(input);
		if (markersPlugin) {
			markersPlugin.setMarkers(data);
		} else {
			markersPlugin = createSeriesMarkers(series, data);
		}
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
		render(candles);
		renderMarkers(markers);

		// Disposing the chart tears down its ResizeObserver and DOM nodes, so
		// repeated mount/unmount cycles do not leak.
		return () => {
			chart?.remove();
			chart = undefined;
			series = undefined;
			markersPlugin = undefined;
			fitted = false;
		};
	});

	// Re-render whenever `candles` changes (no-op until the series exists).
	$effect(() => {
		render(candles);
	});

	// Re-place markers whenever `markers` changes.
	$effect(() => {
		renderMarkers(markers);
	});
</script>

<div bind:this={container} data-testid="chart" class="h-full min-h-[280px] w-full"></div>

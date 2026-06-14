<script lang="ts">
	import { onMount } from 'svelte';
	import {
		createChart,
		createSeriesMarkers,
		CandlestickSeries,
		LineSeries,
		ColorType,
		type CandlestickData,
		type IChartApi,
		type ISeriesApi,
		type ISeriesMarkersPluginApi,
		type MouseEventParams,
		type Time
	} from 'lightweight-charts';
	import type { Candle } from '$lib/api/types';
	import { isIncrementalAppend, toSeriesData } from './candles';
	import { toSeriesMarkers, type ChartMarker } from './markers';
	import { toLineData, type ChartOverlay } from './overlays';

	interface Props {
		/**
		 * OHLC candles to render. The component is source-agnostic: live and
		 * backtest views both feed it the same shape (decision markers and
		 * indicator overlays land here in later lots).
		 */
		candles?: Candle[];
		/** Buy/sell annotations placed at each decision/order timestamp. */
		markers?: ChartMarker[];
		/** Indicator lines (SMA/EMA/Bollinger…) drawn over the price series. */
		overlays?: ChartOverlay[];
		/**
		 * Indicator lines drawn in their own stacked panes below the price, each
		 * with its own scale (RSI/ADX/ATR/DMI…). The time scale is shared, so
		 * zoom/pan stays synchronised with the price pane (#24).
		 */
		subcharts?: ChartOverlay[];
		/** Override the system colour-scheme preference (mostly for tests). */
		dark?: boolean;
		/**
		 * Crosshair hover callback. Fires with the bar `time` under the cursor and
		 * its pixel position, or `null` when the cursor leaves the data area. The
		 * live view uses this to show a decision tooltip (#21).
		 */
		onHover?: (info: HoverInfo | null) => void;
		/**
		 * Click callback. Fires with the clicked bar `time` + pixel position, or
		 * `null` when the click misses the data area. Reuses {@link HoverInfo}: the
		 * live view uses it to open a decision detail panel (#22).
		 */
		onSelect?: (info: HoverInfo | null) => void;
	}

	/** Bar time + pixel position reported on crosshair move / click. */
	export interface HoverInfo {
		time: Time;
		x: number;
		y: number;
	}

	let {
		candles = [],
		markers = [],
		overlays = [],
		subcharts = [],
		dark,
		onHover,
		onSelect
	}: Props = $props();

	let container: HTMLDivElement;
	let chart: IChartApi | undefined;
	let series: ISeriesApi<'Candlestick'> | undefined;
	let markersPlugin: ISeriesMarkersPluginApi<Time> | undefined;
	// Overlay line series kept by id so we can add/update/remove incrementally.
	// A plain record (not a Map) keeps `svelte/prefer-svelte-reactivity` happy.
	let overlaySeries: Record<string, ISeriesApi<'Line'>> = {};
	// Sub-chart line series kept by id, each with the pane index it lives in, so
	// we can add/update/move/remove them and trim emptied panes incrementally.
	let subchartSeries: Record<string, { series: ISeriesApi<'Line'>; pane: number }> = {};
	// Fit the viewport only on the first load; later updates (live polling, #26)
	// must not clobber the user's zoom/pan.
	let fitted = false;
	// Last data pushed to the candlestick series, so a poll can grow it with
	// `update()` instead of a full `setData()`.
	let rendered: CandlestickData[] = [];

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
		if (isIncrementalAppend(rendered, data)) {
			// Live poll: re-apply the boundary bar (it may have updated) and append
			// the new bars via `update()` rather than re-setting the whole series.
			for (let i = rendered.length - 1; i < data.length; i++) series.update(data[i]);
		} else {
			series.setData(data);
			if (!fitted && data.length > 0) {
				chart?.timeScale().fitContent();
				fitted = true;
			}
		}
		rendered = data;
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

	/**
	 * Reconcile overlay line series with the `overlays` prop: add new ids,
	 * update data/colour of existing ones, and remove series that disappeared.
	 * No-op until the chart exists.
	 */
	function renderOverlays(input: ChartOverlay[]): void {
		if (!chart) return;
		const seen = input.map((o) => o.id);
		for (const overlay of input) {
			let line = overlaySeries[overlay.id];
			if (!line) {
				line = chart.addSeries(LineSeries, {
					color: overlay.color,
					lineWidth: 2,
					title: overlay.title ?? overlay.id,
					priceLineVisible: false,
					lastValueVisible: false
				});
				overlaySeries[overlay.id] = line;
			} else {
				line.applyOptions({ color: overlay.color, title: overlay.title ?? overlay.id });
			}
			line.setData(toLineData(overlay.data));
		}
		for (const id of Object.keys(overlaySeries)) {
			if (!seen.includes(id)) {
				chart.removeSeries(overlaySeries[id]);
				delete overlaySeries[id];
			}
		}
	}

	/**
	 * Reconcile the sub-chart line series with the `subcharts` prop. Each enabled
	 * indicator gets its own pane (price is pane 0, indicators are panes 1..N in
	 * order): new ids are added, existing ones updated and moved to their target
	 * pane, vanished ones removed, and emptied trailing panes trimmed. The price
	 * pane is given a larger stretch factor so the sub-charts stay compact.
	 * No-op until the chart exists.
	 */
	function renderSubcharts(input: ChartOverlay[]): void {
		if (!chart) return;
		const seen = input.map((o) => o.id);
		input.forEach((overlay, i) => {
			const pane = i + 1; // pane 0 is the price series
			let entry = subchartSeries[overlay.id];
			if (!entry) {
				const line = chart!.addSeries(
					LineSeries,
					{
						color: overlay.color,
						lineWidth: 2,
						title: overlay.title ?? overlay.id,
						priceLineVisible: false,
						lastValueVisible: false
					},
					pane
				);
				entry = { series: line, pane };
				subchartSeries[overlay.id] = entry;
			} else {
				entry.series.applyOptions({ color: overlay.color, title: overlay.title ?? overlay.id });
				if (entry.pane !== pane) {
					entry.series.moveToPane(pane);
					entry.pane = pane;
				}
			}
			entry.series.setData(toLineData(overlay.data));
		});
		for (const id of Object.keys(subchartSeries)) {
			if (!seen.includes(id)) {
				chart.removeSeries(subchartSeries[id].series);
				delete subchartSeries[id];
			}
		}
		// Trim panes left empty after a removal. Safe top-down: the move/remove
		// pass above has already compacted every surviving series into panes
		// `[1..input.length]`, so any pane with index > input.length is now empty.
		const panes = chart.panes();
		for (let idx = panes.length - 1; idx > input.length; idx--) {
			chart.removePane(idx);
		}
		// Keep the price pane dominant when sub-charts are present.
		if (input.length > 0) {
			const live = chart.panes();
			live[0]?.setStretchFactor(3);
			for (let idx = 1; idx < live.length; idx++) live[idx]?.setStretchFactor(1);
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
		renderOverlays(overlays);
		renderSubcharts(subcharts);

		// Report crosshair hover so the live view can show a decision tooltip.
		// `point`/`time` are undefined once the cursor leaves the data area.
		chart.subscribeCrosshairMove((param: MouseEventParams<Time>) => {
			if (!param.point || param.time === undefined) {
				onHover?.(null);
				return;
			}
			onHover?.({ time: param.time, x: param.point.x, y: param.point.y });
		});

		// Report clicks so the live view can open a decision detail panel. Same
		// payload as hover; `point`/`time` are undefined when the click misses data.
		chart.subscribeClick((param: MouseEventParams<Time>) => {
			if (!param.point || param.time === undefined) {
				onSelect?.(null);
				return;
			}
			onSelect?.({ time: param.time, x: param.point.x, y: param.point.y });
		});

		// Disposing the chart tears down its ResizeObserver and DOM nodes, so
		// repeated mount/unmount cycles do not leak.
		return () => {
			chart?.remove();
			chart = undefined;
			series = undefined;
			markersPlugin = undefined;
			overlaySeries = {};
			subchartSeries = {};
			rendered = [];
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

	// Reconcile overlay lines whenever `overlays` changes.
	$effect(() => {
		renderOverlays(overlays);
	});

	// Reconcile sub-chart panes whenever `subcharts` changes.
	$effect(() => {
		renderSubcharts(subcharts);
	});
</script>

<div bind:this={container} data-testid="chart" class="h-full min-h-[280px] w-full"></div>

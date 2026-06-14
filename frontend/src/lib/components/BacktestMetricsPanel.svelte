<script lang="ts">
	import type { BacktestMetrics } from '$lib/api/types';

	interface Props {
		metrics: BacktestMetrics;
	}

	let { metrics }: Props = $props();

	// Compact number formatting; PnL/fees are quote-currency figures.
	const nf = new Intl.NumberFormat('fr-FR', { maximumFractionDigits: 2 });
	function fmt(n: number): string {
		return nf.format(n);
	}

	const pnlClass = $derived(
		metrics.net_pnl > 0
			? 'text-emerald-400'
			: metrics.net_pnl < 0
				? 'text-rose-400'
				: 'text-slate-100'
	);
</script>

<dl class="grid grid-cols-2 gap-x-6 gap-y-2 text-sm sm:grid-cols-4" data-testid="run-metrics">
	<div>
		<dt class="text-xs text-slate-400">PnL net</dt>
		<dd class="font-mono {pnlClass}" data-testid="net-pnl">{fmt(metrics.net_pnl)}</dd>
	</div>
	<div>
		<dt class="text-xs text-slate-400">PnL brut</dt>
		<dd class="font-mono text-slate-100">{fmt(metrics.gross_pnl)}</dd>
	</div>
	<div>
		<dt class="text-xs text-slate-400">Frais</dt>
		<dd class="font-mono text-slate-100">{fmt(metrics.total_fees)}</dd>
	</div>
	<div>
		<dt class="text-xs text-slate-400">Fills (achat / vente)</dt>
		<dd class="font-mono text-slate-100">{metrics.buy_fills} / {metrics.sell_fills}</dd>
	</div>
</dl>
<p class="mt-2 text-xs text-slate-500">
	PnL cash-flow recalculé depuis les fills (vente − achat − frais). Indicatif : un run non clôturé à
	plat porte une position résiduelle non réalisée.
</p>

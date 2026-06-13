<script lang="ts">
	// Static "indicative results" banner shown on every backtest detail view.
	// The replayed metrics carry well-known biases (cf. robot_rust
	// docs/backtester.md §3); we surface them so a run is never read as a
	// guaranteed live outcome.
	const biases = [
		'slippage et latence simulés, pas mesurés',
		'frais approximés (barème fixe)',
		'pas de market impact ni de carnet réel',
		'données de marché possiblement incomplètes sur la fenêtre',
		'survivorship / look-ahead selon le scénario'
	];
</script>

<aside
	class="rounded-md border border-amber-500/30 bg-amber-500/10 p-3 text-xs text-amber-200"
	data-testid="bias-banner"
	role="note"
>
	<p class="font-medium">Résultats indicatifs</p>
	<p class="mt-1 text-amber-200/80">
		Un backtest est une simulation : {biases.join(' · ')}.
	</p>
</aside>

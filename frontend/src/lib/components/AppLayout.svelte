<script lang="ts">
	import type { Snippet } from 'svelte';

	// Extracted top-level shell so it can be tested in isolation with vitest-browser-svelte.
	// The route +layout.svelte simply forwards its children snippet into this component.
	interface Props {
		children?: Snippet;
	}

	let { children }: Props = $props();
</script>

<div
	class="flex min-h-screen w-full flex-col bg-slate-950 text-slate-100 lg:flex-row"
	data-testid="app-shell"
>
	<nav
		aria-label="Sélecteurs"
		class="w-full shrink-0 border-b border-slate-800 bg-slate-900 p-4 lg:w-[280px] lg:border-r lg:border-b-0"
		data-testid="app-sidebar"
	>
		<h1 class="text-lg font-semibold tracking-tight text-slate-100">Pompote View</h1>
		<p class="mt-1 text-xs text-slate-400">Visualisation & monitoring des stratégies de trading</p>

		<div
			class="mt-6 rounded-md border border-dashed border-slate-700 bg-slate-800/40 p-4 text-sm text-slate-400"
			data-testid="selectors-placeholder"
		>
			Sélecteurs (à venir)
		</div>
	</nav>

	<main
		id="chart-main"
		aria-label="Zone d'affichage du graphique"
		class="flex min-h-[60vh] flex-1 items-center justify-center p-6 lg:min-h-screen"
		data-testid="app-main"
	>
		{#if children}
			{@render children()}
		{/if}
	</main>
</div>

<script lang="ts">
	import './layout.css';
	import { QueryClient, QueryClientProvider } from '@tanstack/svelte-query';
	import favicon from '$lib/assets/favicon.svg';
	import AppLayout from '$lib/components/AppLayout.svelte';

	let { children } = $props();

	// One client for the app: powers caching, dedup and the 10s polling the
	// live monitoring views use (Lot 7). Browser calls go to same-origin
	// `/api/*` proxy routes — the X-API-Key stays server-side.
	const queryClient = new QueryClient();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<title>Pompote View</title>
</svelte:head>

<QueryClientProvider client={queryClient}>
	<AppLayout {children} />
</QueryClientProvider>

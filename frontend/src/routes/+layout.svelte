<script lang="ts">
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';
	import AppLayout from '$lib/components/AppLayout.svelte';

	let { children } = $props();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<title>Pompote View</title>
</svelte:head>

<AppLayout {children} />

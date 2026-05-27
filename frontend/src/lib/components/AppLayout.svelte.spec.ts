import { page } from 'vitest/browser';
import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { createRawSnippet } from 'svelte';
import AppLayout from './AppLayout.svelte';

describe('AppLayout.svelte', () => {
	it('renders the sidebar with the app title and the main chart zone with children', async () => {
		// Build a minimal snippet that injects a marker into the main zone, so we can
		// verify that the layout properly wires the `children` snippet through.
		const childContent = createRawSnippet(() => ({
			render: () => '<span data-testid="main-child">Chart placeholder</span>'
		}));

		render(AppLayout, { children: childContent });

		// Sidebar landmark with the brand title.
		const sidebar = page.getByRole('navigation', { name: 'Sélecteurs' });
		await expect.element(sidebar).toBeInTheDocument();
		await expect
			.element(page.getByRole('heading', { level: 1, name: 'Pompote View' }))
			.toBeInTheDocument();

		// Main landmark receives the children snippet.
		const main = page.getByRole('main');
		await expect.element(main).toBeInTheDocument();
		await expect.element(page.getByTestId('main-child')).toHaveTextContent('Chart placeholder');
	});
});

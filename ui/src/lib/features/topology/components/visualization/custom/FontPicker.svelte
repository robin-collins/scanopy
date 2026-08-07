<script lang="ts">
	import { ChevronDown } from 'lucide-svelte';
	import { FONT_CATALOG, searchFonts, loadFont, getFontCssStack } from './fonts';
	import { topology_customViewFontSearchPlaceholder } from '$lib/paraglide/messages';

	interface Props {
		value: string | null;
		onSelect: (fontId: string | null) => void;
		label?: string;
	}

	let { value, onSelect, label = 'Font' }: Props = $props();

	let open = $state(false);
	let search = $state('');
	let results = $derived(searchFonts(search));

	// Preload every catalog font's regular weight while the picker is open so
	// each option previews in its own typeface — cheap since options are
	// individually small, and this only happens on user-initiated picker use.
	$effect(() => {
		if (open) for (const font of FONT_CATALOG) loadFont(font.id);
	});

	function select(fontId: string | null) {
		onSelect(fontId);
		open = false;
		search = '';
	}
</script>

<div class="relative">
	<span class="block text-xs font-medium">{label}</span>
	<button
		type="button"
		class="input-field mt-1 flex w-full items-center justify-between text-left"
		style:font-family={getFontCssStack(value)}
		onclick={() => (open = !open)}
	>
		<span class="truncate">{value ?? 'System default'}</span>
		<ChevronDown class="h-3.5 w-3.5 flex-shrink-0" />
	</button>

	{#if open}
		<div
			class="absolute z-20 mt-1 w-full rounded-md border border-gray-200 bg-white shadow-lg dark:border-gray-700 dark:bg-gray-900"
		>
			<input
				class="input-field w-full rounded-b-none border-x-0 border-t-0"
				placeholder={topology_customViewFontSearchPlaceholder()}
				bind:value={search}
				onkeydown={(e) => e.stopPropagation()}
			/>
			<div class="max-h-56 overflow-y-auto p-1">
				<button
					type="button"
					class="block w-full rounded px-2 py-1.5 text-left text-xs hover:bg-gray-100 dark:hover:bg-gray-800"
					onclick={() => select(null)}
				>
					System default
				</button>
				{#each results as font (font.id)}
					<button
						type="button"
						class="block w-full rounded px-2 py-1.5 text-left text-sm hover:bg-gray-100 dark:hover:bg-gray-800"
						style:font-family={getFontCssStack(font.id)}
						onclick={() => select(font.id)}
					>
						{font.id}
					</button>
				{:else}
					<div class="px-2 py-1.5 text-xs text-gray-400">No fonts match "{search}"</div>
				{/each}
			</div>
		</div>
	{/if}
</div>

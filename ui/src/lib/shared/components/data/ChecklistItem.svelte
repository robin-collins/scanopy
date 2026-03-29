<script lang="ts">
	import { Check, Circle } from 'lucide-svelte';
	import type { Snippet } from 'svelte';

	interface Props {
		checked?: boolean;
		onToggle?: () => void;
		disabled?: boolean;
		label: string;
		description?: string;
		/** Custom icon snippet — overrides the default Check/Circle icons */
		icon?: Snippet;
		/** Detail content shown when NOT checked (collapses on check) */
		detail?: Snippet;
		/** Extra content after the label (badges, tags, etc.) */
		labelExtra?: Snippet;
		/** Sub-content below the label (shown regardless of checked state) */
		subContent?: Snippet;
		/** Render in a card container */
		card?: boolean;
		class?: string;
	}

	let {
		checked = false,
		onToggle,
		disabled = false,
		label,
		description,
		icon,
		detail,
		labelExtra,
		subContent,
		card = false,
		class: className = ''
	}: Props = $props();

	function handleClick() {
		if (!disabled && onToggle) {
			onToggle();
		}
	}
</script>

<div class="{card ? 'card card-static' : ''} {className}">
	<button
		type="button"
		class="flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left transition-colors
			{!checked && !disabled ? 'hover:bg-gray-100 dark:hover:bg-gray-800/50' : ''}
			{disabled ? 'opacity-50' : ''}"
		{disabled}
		onclick={handleClick}
	>
		{#if icon}
			{@render icon()}
		{:else if checked}
			<Check class="h-5 w-5 flex-shrink-0 text-green-400" />
		{:else}
			<Circle class="h-5 w-5 flex-shrink-0 {disabled ? 'text-disabled' : 'text-tertiary'}" />
		{/if}
		<div class="min-w-0 flex-1">
			<div class="flex items-center gap-2">
				<span
					class="text-sm font-medium"
					class:text-primary={!checked && !disabled}
					class:text-tertiary={checked}
					class:text-disabled={!checked && disabled}
					class:line-through={checked}
				>
					{label}
				</span>
				{#if labelExtra}
					{@render labelExtra()}
				{/if}
			</div>
			{#if description && !checked}
				<p class="text-tertiary text-xs">{description}</p>
			{/if}
			{#if subContent}
				{@render subContent()}
			{/if}
		</div>
	</button>

	{#if detail && !checked}
		<div class="ml-11 mt-1 space-y-2 pb-1">
			{@render detail()}
		</div>
	{/if}
</div>

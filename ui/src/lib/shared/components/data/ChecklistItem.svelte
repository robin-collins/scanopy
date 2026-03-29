<script lang="ts">
	import { Check, Circle } from 'lucide-svelte';
	import type { Snippet } from 'svelte';

	interface Props {
		checked?: boolean;
		onToggle?: () => void;
		disabled?: boolean;
		label: string;
		/** Custom icon snippet — overrides the default Check/Circle icons */
		icon?: Snippet;
		/** Detail content shown when NOT checked (collapses on check) */
		detail?: Snippet;
		/** Extra content after the label (badges, tags, etc.) */
		labelExtra?: Snippet;
		/** Sub-content below the label (shown regardless of checked state) */
		subContent?: Snippet;
		class?: string;
	}

	let {
		checked = false,
		onToggle,
		disabled = false,
		label,
		icon,
		detail,
		labelExtra,
		subContent,
		class: className = ''
	}: Props = $props();

	function handleClick() {
		if (!disabled && onToggle) {
			onToggle();
		}
	}
</script>

<div class={className}>
	<button
		type="button"
		class="flex w-full items-center gap-2.5 py-2 text-left transition-colors
			{!checked && !disabled ? 'hover:opacity-80' : ''}
			{disabled ? 'opacity-50' : ''}"
		{disabled}
		onclick={handleClick}
	>
		{#if icon}
			{@render icon()}
		{:else if checked}
			<Check class="h-4 w-4 flex-shrink-0 text-green-400" />
		{:else}
			<Circle class="h-4 w-4 flex-shrink-0 {disabled ? 'text-disabled' : 'text-tertiary'}" />
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
			{#if subContent}
				{@render subContent()}
			{/if}
		</div>
	</button>

	{#if detail && !checked}
		<div class="ml-6.5 border-l border-gray-200 pb-3 pl-4 dark:border-gray-700">
			{@render detail()}
		</div>
	{/if}
</div>

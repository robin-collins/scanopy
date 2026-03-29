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

	let hasDetail = $derived(!!detail && !checked);
</script>

<div class="flex gap-3 {className}">
	<!-- Icon column with vertical connector line -->
	<div class="flex flex-col items-center">
		<button
			type="button"
			class="flex-shrink-0 pt-3 transition-colors {!disabled ? 'cursor-pointer hover:opacity-70' : 'opacity-50'}"
			{disabled}
			onclick={handleClick}
		>
			{#if icon}
				{@render icon()}
			{:else if checked}
				<Check class="h-5 w-5 text-green-400" />
			{:else}
				<Circle class="h-5 w-5 {disabled ? 'text-disabled' : 'text-tertiary'}" />
			{/if}
		</button>
		{#if hasDetail}
			<div class="mt-1 w-px flex-1 bg-gray-200 dark:bg-gray-700"></div>
		{/if}
	</div>

	<!-- Content column -->
	<div class="min-w-0 flex-1 pb-1">
		<button
			type="button"
			class="flex w-full items-center gap-2 pt-3 text-left {!disabled ? 'cursor-pointer' : 'opacity-50'}"
			{disabled}
			onclick={handleClick}
		>
			<span
				class="text-base font-medium"
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
		</button>
		{#if subContent}
			{@render subContent()}
		{/if}

		{#if hasDetail}
			<div class="space-y-2.5 pb-2 pt-2 text-sm">
				{@render detail()}
			</div>
		{/if}
	</div>
</div>

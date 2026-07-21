<script lang="ts">
	import { Handle, Position, type NodeProps } from '@xyflow/svelte';
	import { createColorHelper, createIconComponent } from '$lib/shared/utils/styling';
	import type { CustomObjectNodeData } from './types';

	let { data, selected }: NodeProps & { data: CustomObjectNodeData } = $props();

	let style = $derived(data.view.style ?? 'Image');
	let colorStyle = $derived(createColorHelper(data.view.color ?? null));
	let IconComponent = $derived(data.icon ? createIconComponent(data.icon) : null);
	let badgeText = $derived((data.view.badge_text || data.label.slice(0, 2) || '?').toUpperCase());
</script>

<div class="custom-object-node group relative flex flex-col items-center gap-1" class:selected>
	<Handle type="target" id="Top" position={Position.Top} class="node-handle" />
	<Handle type="target" id="Left" position={Position.Left} class="node-handle" />

	{#if style === 'Badge'}
		<div
			class="flex h-14 w-14 items-center justify-center rounded-full border-2 text-lg font-bold {colorStyle.bg} {colorStyle.text} {colorStyle.border}"
		>
			{badgeText}
		</div>
	{:else if style === 'StatsCard'}
		<div
			class="min-w-[160px] rounded-lg border bg-white p-2 shadow-sm dark:bg-gray-800 {colorStyle.border}"
		>
			<div class="flex items-center gap-2">
				{#if data.imageUrl}
					<img src={data.imageUrl} alt="" class="h-8 w-8 rounded object-cover" />
				{:else if IconComponent}
					<IconComponent class="h-6 w-6 {colorStyle.icon}" />
				{/if}
				<span class="truncate text-sm font-medium">{data.label}</span>
			</div>
			{#if data.stats && data.stats.length > 0}
				<ul class="mt-1 space-y-0.5 text-xs text-gray-500 dark:text-gray-400">
					{#each data.stats as stat (stat.label)}
						<li class="flex justify-between gap-2">
							<span>{stat.label}</span>
							<span class="font-mono">{stat.value}</span>
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	{:else if style === 'ImageBordered'}
		<div
			class="flex h-16 w-16 items-center justify-center overflow-hidden rounded-lg border-2 bg-white dark:bg-gray-800 {colorStyle.border}"
		>
			{#if data.imageUrl}
				<img src={data.imageUrl} alt="" class="h-full w-full object-cover" />
			{:else if IconComponent}
				<IconComponent class="h-8 w-8 {colorStyle.icon}" />
			{/if}
		</div>
	{:else}
		<!-- Image (default): no frame, just the glyph -->
		<div class="flex h-14 w-14 items-center justify-center">
			{#if data.imageUrl}
				<img src={data.imageUrl} alt="" class="h-full w-full rounded object-cover" />
			{:else if IconComponent}
				<IconComponent class="h-10 w-10 {colorStyle.icon}" />
			{/if}
		</div>
	{/if}

	{#if style !== 'StatsCard'}
		<span
			class="max-w-[120px] truncate rounded bg-white/80 px-1 text-xs font-medium text-gray-700 dark:bg-gray-900/80 dark:text-gray-200"
		>
			{data.label}
		</span>
	{/if}

	<Handle type="source" id="Bottom" position={Position.Bottom} class="node-handle" />
	<Handle type="source" id="Right" position={Position.Right} class="node-handle" />
</div>

<style>
	.custom-object-node.selected {
		outline: 2px solid var(--color-primary, #3b82f6);
		outline-offset: 2px;
		border-radius: 0.5rem;
	}

	:global(.node-handle) {
		width: 8px;
		height: 8px;
		background: var(--color-primary, #3b82f6);
		opacity: 0;
		transition: opacity 0.15s;
	}

	.custom-object-node:hover :global(.node-handle) {
		opacity: 1;
	}
</style>

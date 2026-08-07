<script lang="ts">
	import { Handle, NodeResizer, Position, type NodeProps } from '@xyflow/svelte';
	import { getNodeAppearance, getSafeCanvasLink } from './custom-view-model';
	import {
		common_openLink,
		topology_customViewGroupNamePlaceholder
	} from '$lib/paraglide/messages';
	import type { CustomGroupNodeData } from './types';

	let { data, selected }: NodeProps & { data: CustomGroupNodeData } = $props();

	let appearance = $derived(getNodeAppearance(data.view));
	const HANDLE_POSITIONS = [Position.Top, Position.Right, Position.Bottom, Position.Left];

	let label = $state('');
	let lastNodeId = $state<string | null>(null);
	$effect(() => {
		if (data.view.id !== lastNodeId) {
			lastNodeId = data.view.id;
			label = data.view.label ?? '';
		}
	});

	function handleLabelBlur() {
		if (label !== (data.view.label ?? '')) {
			data.onLabelChange(label);
		}
	}

	function handleResizeEnd(_event: unknown, params: { width: number; height: number }) {
		data.onResizeEnd(params.width, params.height);
	}

	function stopCanvasInteraction(event: Event) {
		event.stopPropagation();
	}
</script>

<NodeResizer
	minWidth={120}
	minHeight={80}
	isVisible={selected}
	color="#3b82f6"
	handleClass="!h-3 !w-3 !rounded-full !border-2 !border-white"
	onResizeEnd={handleResizeEnd}
/>

<div
	class="custom-group-node relative h-full w-full border-2"
	style:color={appearance.primary}
	style:background-color={appearance.background}
	style:opacity={appearance.opacity}
	style:border-color={appearance.secondary}
	style:border-style={appearance.borderStyle}
	style:border-radius={appearance.borderRadius}
	style:font-family={appearance.fontFamily}
	style:font-size={`${appearance.fontSize}px`}
	style:font-weight={appearance.fontWeight}
	style:font-style={appearance.fontStyle}
	style:text-decoration={appearance.textDecoration}
	style:text-align={appearance.textAlign}
>
	{#each HANDLE_POSITIONS as position (position)}
		<Handle type="source" id="handle-{position}" {position} class="node-handle" />
	{/each}
	{#if data.view.show_label !== false}
		<input
			class="nodrag nopan absolute -top-3 left-3 rounded bg-white px-1.5 py-0.5 dark:bg-gray-900"
			value={label}
			oninput={(e) => (label = (e.target as HTMLInputElement).value)}
			onblur={handleLabelBlur}
			onmousedown={stopCanvasInteraction}
			onpointerdown={stopCanvasInteraction}
			onclick={stopCanvasInteraction}
			onkeydown={stopCanvasInteraction}
			placeholder={topology_customViewGroupNamePlaceholder()}
		/>
	{/if}
	{#if data.view.show_description !== false && data.view.description}
		<p
			class="nodrag nopan pointer-events-none absolute left-3 top-4 max-w-[calc(100%-1.5rem)] truncate text-xs opacity-80"
		>
			{data.view.description}
		</p>
	{/if}
	{#if getSafeCanvasLink(data.view.link_url)}
		<button
			type="button"
			class="nodrag absolute right-2 top-1 underline"
			title={common_openLink()}
			onclick={() =>
				window.open(getSafeCanvasLink(data.view.link_url)!, '_blank', 'noopener,noreferrer')}
			>↗</button
		>
	{/if}
</div>

<style>
	.custom-group-node {
		opacity: 0.9;
	}
</style>

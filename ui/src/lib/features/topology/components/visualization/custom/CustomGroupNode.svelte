<script lang="ts">
	import { NodeResizer, type NodeProps } from '@xyflow/svelte';
	import { createColorHelper } from '$lib/shared/utils/styling';
	import { topology_customViewGroupNamePlaceholder } from '$lib/paraglide/messages';
	import type { CustomGroupNodeData } from './types';

	let { data, selected, width, height }: NodeProps & { data: CustomGroupNodeData } = $props();

	let colorStyle = $derived(createColorHelper(data.view.color ?? null));
	let cornerClass = $derived(data.view.corner_style === 'Square' ? 'rounded-none' : 'rounded-lg');

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
</script>

<NodeResizer
	minWidth={120}
	minHeight={80}
	isVisible={selected}
	lineClass="!border-transparent"
	handleClass="!bg-transparent !border-transparent"
	onResizeEnd={handleResizeEnd}
/>

<div
	class="custom-group-node h-full w-full border-2 {cornerClass} {colorStyle.bg} {colorStyle.border}"
	class:selected
	style:width={width ? `${width}px` : undefined}
	style:height={height ? `${height}px` : undefined}
>
	<input
		class="nodrag nopan absolute -top-3 left-3 rounded bg-white px-1.5 py-0.5 text-xs font-semibold dark:bg-gray-900 {colorStyle.text}"
		value={label}
		oninput={(e) => (label = (e.target as HTMLInputElement).value)}
		onblur={handleLabelBlur}
		placeholder={topology_customViewGroupNamePlaceholder()}
	/>
</div>

<style>
	.custom-group-node {
		opacity: 0.9;
	}

	.custom-group-node.selected {
		box-shadow: 0 0 0 2px var(--color-primary, #3b82f6);
	}
</style>

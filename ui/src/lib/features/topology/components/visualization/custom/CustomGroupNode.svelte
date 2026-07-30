<script lang="ts">
	import { NodeResizer, type NodeProps } from '@xyflow/svelte';
	import { createColorHelper } from '$lib/shared/utils/styling';
	import { topology_customViewGroupNamePlaceholder } from '$lib/paraglide/messages';
	import type { CustomGroupNodeData } from './types';

	let { data, selected }: NodeProps & { data: CustomGroupNodeData } = $props();

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
	class="custom-group-node relative h-full w-full border-2 {cornerClass} {colorStyle.bg} {colorStyle.border}"
>
	<input
		class="nodrag nopan absolute -top-3 left-3 rounded bg-white px-1.5 py-0.5 text-xs font-semibold dark:bg-gray-900 {colorStyle.text}"
		value={label}
		oninput={(e) => (label = (e.target as HTMLInputElement).value)}
		onblur={handleLabelBlur}
		onmousedown={stopCanvasInteraction}
		onpointerdown={stopCanvasInteraction}
		onclick={stopCanvasInteraction}
		onkeydown={stopCanvasInteraction}
		placeholder={topology_customViewGroupNamePlaceholder()}
	/>
</div>

<style>
	.custom-group-node {
		opacity: 0.9;
	}
</style>

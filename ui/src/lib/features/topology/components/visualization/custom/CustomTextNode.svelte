<script lang="ts">
	import { Handle, NodeResizer, Position, type NodeProps } from '@xyflow/svelte';
	import { getNodeAppearance, getSafeCanvasLink } from './custom-view-model';
	import type { CustomTextNodeData } from './types';

	let { data, selected }: NodeProps & { data: CustomTextNodeData } = $props();

	const HANDLE_POSITIONS = [Position.Top, Position.Right, Position.Bottom, Position.Left];
	let appearance = $derived(getNodeAppearance(data.view));

	let text = $state('');
	// Local edits shouldn't be clobbered by a query refetch mid-typing, but
	// once the node identity changes (a different text node mounted into this
	// same component instance) re-seed from the latest server value.
	let lastNodeId = $state<string | null>(null);
	$effect(() => {
		if (data.view.id !== lastNodeId) {
			lastNodeId = data.view.id;
			text = data.view.text_content ?? '';
		}
	});

	function handleBlur() {
		if (text !== (data.view.text_content ?? '')) {
			data.onTextChange(text);
		}
	}

	function stopCanvasInteraction(event: Event) {
		event.stopPropagation();
	}
</script>

<NodeResizer
	minWidth={80}
	minHeight={40}
	isVisible={selected}
	onResizeEnd={(_event, params) => data.onResizeEnd(params.width, params.height)}
/>

<div class="custom-text-node h-full w-full" class:selected style:opacity={appearance.opacity}>
	{#each HANDLE_POSITIONS as position (position)}
		<Handle type="source" id="handle-{position}" {position} class="node-handle" />
	{/each}

	<div
		role="textbox"
		tabindex="0"
		contenteditable="true"
		class="nodrag nopan min-h-[2rem] min-w-[6rem] max-w-[30rem] whitespace-pre-wrap rounded p-2 outline-none"
		style:color={appearance.primary}
		style:background-color={appearance.background}
		style:font-family={appearance.fontFamily}
		style:font-size={`${appearance.fontSize}px`}
		style:font-weight={appearance.fontWeight}
		style:font-style={appearance.fontStyle}
		style:border={`2px ${appearance.borderStyle} ${appearance.secondary}`}
		style:border-radius={appearance.borderRadius}
		bind:textContent={text}
		onblur={handleBlur}
		onmousedown={stopCanvasInteraction}
		onpointerdown={stopCanvasInteraction}
		onclick={stopCanvasInteraction}
		onkeydown={stopCanvasInteraction}
	></div>
	{#if getSafeCanvasLink(data.view.link_url)}
		<button
			type="button"
			class="nodrag absolute right-1 top-1 z-10 text-xs underline"
			title="Open link"
			onclick={() =>
				window.open(getSafeCanvasLink(data.view.link_url)!, '_blank', 'noopener,noreferrer')}
			>↗</button
		>
	{/if}
</div>

<style>
	.custom-text-node.selected {
		outline: 2px solid var(--color-primary, #3b82f6);
		outline-offset: 2px;
		border-radius: 0.375rem;
	}

	:global(.node-handle) {
		width: 8px;
		height: 8px;
		background: var(--color-primary, #3b82f6);
		opacity: 0;
		transition: opacity 0.15s;
	}

	.custom-text-node:hover :global(.node-handle) {
		opacity: 1;
	}
</style>

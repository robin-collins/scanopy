<script lang="ts">
	import { Handle, Position, type NodeProps } from '@xyflow/svelte';
	import type { CustomTextNodeData } from './types';

	let { data, selected }: NodeProps & { data: CustomTextNodeData } = $props();

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
</script>

<div class="custom-text-node" class:selected>
	<Handle type="target" id="Top" position={Position.Top} class="node-handle" />
	<Handle type="target" id="Left" position={Position.Left} class="node-handle" />

	<div
		role="textbox"
		tabindex="0"
		contenteditable="true"
		class="nodrag nopan min-h-[2rem] min-w-[6rem] max-w-[20rem] whitespace-pre-wrap rounded p-2 text-sm text-gray-800 outline-none dark:text-gray-100"
		bind:textContent={text}
		onblur={handleBlur}
	></div>

	<Handle type="source" id="Bottom" position={Position.Bottom} class="node-handle" />
	<Handle type="source" id="Right" position={Position.Right} class="node-handle" />
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

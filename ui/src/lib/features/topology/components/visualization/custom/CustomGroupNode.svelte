<script lang="ts">
	import { Handle, NodeResizer, Position, type NodeProps, type ResizeParams } from '@xyflow/svelte';
	import { getNodeAppearance, getSafeCanvasLink } from './custom-view-model';
	import {
		common_openLink,
		topology_customViewGroupNamePlaceholder
	} from '$lib/paraglide/messages';
	import type { CustomGroupNodeData } from './types';

	let { data, selected }: NodeProps & { data: CustomGroupNodeData } = $props();

	let appearance = $derived(getNodeAppearance(data.view, data.canvasDefaults));
	const HANDLE_POSITIONS = [Position.Top, Position.Right, Position.Bottom, Position.Left];
	let label = $state('');
	let labelSource = $state('');
	let editingLabel = $state(false);
	let lastNodeId = $state<string | null>(null);

	$effect(() => {
		const nextLabel = data.view.label ?? '';
		if (data.view.id !== lastNodeId) {
			lastNodeId = data.view.id;
			label = nextLabel;
			labelSource = nextLabel;
			editingLabel = false;
			return;
		}
		if (nextLabel !== labelSource) {
			labelSource = nextLabel;
			if (!editingLabel) label = nextLabel;
		}
	});

	function handleLabelBlur() {
		editingLabel = false;
		if (label !== (data.view.label ?? '')) data.onLabelChange(label);
	}

	function handleResizeEnd(_event: unknown, params: ResizeParams) {
		data.onResizeEnd(params);
	}

	function stopCanvasInteraction(event: Event) {
		event.stopPropagation();
	}

	function focusEditor(node: HTMLTextAreaElement) {
		node.focus();
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
	class="custom-group-node relative h-full w-full overflow-hidden border-2"
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
	<div
		class="group-copy pointer-events-none absolute inset-0 flex min-h-0 flex-col gap-1 overflow-hidden p-2"
	>
		{#if data.view.show_label !== false}
			{#if editingLabel}
				<textarea
					use:focusEditor
					class="group-label-editor nodrag nopan pointer-events-auto rounded bg-white/90 px-1.5 py-0.5 outline-none dark:bg-gray-900/90"
					rows="3"
					maxlength={200}
					bind:value={label}
					onblur={handleLabelBlur}
					onmousedown={stopCanvasInteraction}
					onpointerdown={stopCanvasInteraction}
					onclick={stopCanvasInteraction}
					onkeydown={stopCanvasInteraction}
					placeholder={topology_customViewGroupNamePlaceholder()}
				></textarea>
			{:else}
				<button
					type="button"
					class="group-label nodrag nopan pointer-events-auto rounded bg-white/90 px-1.5 py-0.5 dark:bg-gray-900/90"
					title={data.view.label ?? ''}
					onclick={(event) => {
						stopCanvasInteraction(event);
						editingLabel = true;
					}}
				>
					{data.view.label || topology_customViewGroupNamePlaceholder()}
				</button>
			{/if}
		{/if}
		{#if data.view.show_description !== false && data.view.description}
			<p class="group-description min-h-0 flex-1 opacity-80">
				{data.view.description}
			</p>
		{/if}
	</div>
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

	.group-copy {
		padding-right: 1.75rem;
	}

	.group-label {
		display: -webkit-box;
		max-height: 40%;
		flex-shrink: 1;
		overflow: hidden;
		overflow-wrap: anywhere;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 3;
		line-clamp: 3;
		line-height: 1.2;
		width: 100%;
		border: 0;
		color: inherit;
		font: inherit;
		text-align: inherit;
	}

	.group-label-editor {
		box-sizing: border-box;
		max-height: 40%;
		width: 100%;
		flex-shrink: 1;
		resize: none;
		overflow: auto;
		color: inherit;
		font: inherit;
		line-height: 1.2;
		text-align: inherit;
	}

	.group-description {
		margin: 0;
		overflow: hidden;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		line-height: 1.25;
	}
</style>

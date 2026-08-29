<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { Handle, NodeResizer, Position, type NodeProps } from '@xyflow/svelte';
	import { common_openLink } from '$lib/paraglide/messages';
	import { getNodeAppearance, getSafeCanvasLink } from './custom-view-model';
	import type { CustomTextNodeData } from './types';
	import { getAutoGrowBounds } from './text-overflow';

	let { data, selected }: NodeProps & { data: CustomTextNodeData } = $props();

	const HANDLE_POSITIONS = [Position.Top, Position.Right, Position.Bottom, Position.Left];
	let appearance = $derived(getNodeAppearance(data.view, data.canvasDefaults));

	let text = $state('');
	let nodeElement: HTMLDivElement | undefined = $state();
	let contentElement: HTMLDivElement | undefined = $state();
	let pendingMeasurement: number | null = null;
	let lastAutoGrowRequest = '';
	let editing = $state(false);
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

	async function handleBlur() {
		if (text !== (data.view.text_content ?? '')) {
			const saved = await data.onTextChange(text);
			if (!saved) text = data.view.text_content ?? '';
		}
		editing = document.activeElement === contentElement;
		if (!editing) scheduleAutoGrow();
	}

	function scheduleAutoGrow() {
		if (editing) return;
		if (pendingMeasurement !== null) cancelAnimationFrame(pendingMeasurement);
		pendingMeasurement = requestAnimationFrame(async () => {
			pendingMeasurement = null;
			await tick();
			if (editing || !nodeElement || !contentElement) return;

			const bounds = getAutoGrowBounds(
				{
					currentWidth: nodeElement.offsetWidth,
					currentHeight: nodeElement.offsetHeight,
					contentWidth:
						nodeElement.offsetWidth +
						Math.max(0, contentElement.scrollWidth - contentElement.clientWidth),
					contentHeight: contentElement.offsetHeight
				},
				{ x: data.view.x, y: data.view.y }
			);
			if (!bounds) {
				lastAutoGrowRequest = '';
				return;
			}

			const requestKey = [
				data.view.id,
				text,
				data.view.font_family,
				data.view.font_size,
				data.view.font_bold,
				data.view.font_italic,
				bounds.width,
				bounds.height
			].join('|');
			if (requestKey === lastAutoGrowRequest) return;
			lastAutoGrowRequest = requestKey;
			data.onAutoGrow(bounds);
		});
	}

	function handleInput(event: Event) {
		text = (event.currentTarget as HTMLDivElement).textContent ?? '';
	}

	function stopCanvasInteraction(event: Event) {
		event.stopPropagation();
	}

	$effect(() => {
		// Re-measure after persisted text or any appearance property changes.
		void `${data.view.text_content}|${data.view.font_family}|${data.view.font_size}|${data.view.font_bold}|${data.view.font_italic}|${data.view.font_underline}`;
		scheduleAutoGrow();
	});

	onMount(() => {
		const observer = new ResizeObserver(scheduleAutoGrow);
		if (nodeElement) observer.observe(nodeElement);
		if (contentElement) observer.observe(contentElement);
		const fonts = document.fonts;
		fonts?.addEventListener('loadingdone', scheduleAutoGrow);
		scheduleAutoGrow();
		return () => {
			observer.disconnect();
			fonts?.removeEventListener('loadingdone', scheduleAutoGrow);
			if (pendingMeasurement !== null) cancelAnimationFrame(pendingMeasurement);
		};
	});
</script>

<NodeResizer
	minWidth={80}
	minHeight={40}
	isVisible={selected}
	onResizeEnd={(_event, params) => data.onResizeEnd(params)}
/>

<div
	bind:this={nodeElement}
	class="custom-text-node relative h-full w-full overflow-hidden"
	class:selected
	class:editing
	style:opacity={appearance.opacity}
>
	{#each HANDLE_POSITIONS as position (position)}
		<Handle type="source" id="handle-{position}" {position} class="node-handle" />
	{/each}

	<div
		bind:this={contentElement}
		role="textbox"
		tabindex="0"
		contenteditable="true"
		class="nodrag nopan box-border min-h-full w-full whitespace-pre-wrap break-words rounded p-2 outline-none"
		style:color={appearance.primary}
		style:background-color={appearance.background}
		style:font-family={appearance.fontFamily}
		style:font-size={`${appearance.fontSize}px`}
		style:font-weight={appearance.fontWeight}
		style:font-style={appearance.fontStyle}
		style:text-decoration={appearance.textDecoration}
		style:text-align={appearance.textAlign}
		style:border={`2px ${appearance.borderStyle} ${appearance.secondary}`}
		style:border-radius={appearance.borderRadius}
		bind:textContent={text}
		onfocus={() => (editing = true)}
		oninput={handleInput}
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
			title={common_openLink()}
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

	.custom-text-node.editing {
		overflow: visible;
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

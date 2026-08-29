<script lang="ts">
	import { Handle, NodeResizer, Position, type NodeProps, type ResizeParams } from '@xyflow/svelte';
	import { createColorHelper, createIconComponent } from '$lib/shared/utils/styling';
	import { serviceDefinitions } from '$lib/shared/stores/metadata';
	import { common_openLink } from '$lib/paraglide/messages';
	import type { CustomObjectNodeData } from './types';
	import { getNodeAppearance, getSafeCanvasLink } from './custom-view-model';

	let { data, selected, width, height }: NodeProps & { data: CustomObjectNodeData } = $props();

	let style = $derived(data.view.style ?? 'Image');
	let colorStyle = $derived(createColorHelper(data.view.color ?? null));
	let IconComponent = $derived(data.icon ? createIconComponent(data.icon) : null);
	let badgeText = $derived((data.view.badge_text || data.label.slice(0, 2) || '?').toUpperCase());
	let appearance = $derived(getNodeAppearance(data.view, data.canvasDefaults));

	function handleResizeEnd(_event: unknown, params: ResizeParams) {
		data.onResizeEnd(params);
	}

	const HANDLE_POSITIONS = [Position.Top, Position.Right, Position.Bottom, Position.Left];
</script>

<NodeResizer
	minWidth={style === 'StatsCard' ? 160 : 60}
	minHeight={style === 'StatsCard' ? 100 : 60}
	isVisible={selected}
	onResizeEnd={handleResizeEnd}
/>

<div
	class="custom-object-node group relative flex h-full w-full flex-col items-center overflow-hidden"
	class:selected
	style:width={width ? `${width}px` : undefined}
	style:height={height ? `${height}px` : undefined}
	style:color={appearance.primary}
	style:background-color={appearance.background}
	style:opacity={appearance.opacity}
	style:border={`2px ${appearance.borderStyle} ${appearance.secondary}`}
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

	{#if style === 'Badge'}
		<div class="flex min-h-0 w-full flex-1 items-center justify-center overflow-hidden">
			<div
				class="flex h-14 w-14 flex-shrink-0 items-center justify-center rounded-full border-2 font-bold {colorStyle.bg} {colorStyle.text} {colorStyle.border}"
			>
				{badgeText}
			</div>
		</div>
	{:else if style === 'StatsCard'}
		<div
			class="flex h-full min-h-0 w-full flex-col overflow-hidden rounded-lg border bg-white shadow-sm dark:bg-gray-800 {colorStyle.border}"
		>
			{#if data.headerText}
				<div
					class="text-tertiary min-w-0 flex-shrink-0 truncate px-2 pt-2 text-center"
					title={data.headerText}
				>
					{data.headerText}
				</div>
			{/if}
			<div class="flex min-w-0 flex-shrink-0 items-center justify-center gap-1.5 px-2 pt-1">
				{#if data.imageUrl}
					<img src={data.imageUrl} alt="" class="h-5 w-5 flex-shrink-0 rounded object-cover" />
				{:else if IconComponent}
					<IconComponent class="h-5 w-5 flex-shrink-0 {colorStyle.icon}" />
				{/if}
				<span class="text-primary min-w-0 truncate" title={data.label}>{data.label}</span>
			</div>
			<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
			<div
				class="flex min-h-0 flex-1 flex-col items-center overflow-y-auto overscroll-contain px-2 py-2"
				tabindex="0"
				role="region"
				aria-label={`${data.label} services`}
			>
				{#each data.services ?? [] as service (service.id)}
					{@const ServiceIcon = serviceDefinitions.getIconComponent(service.service_definition)}
					{@const serviceColor = serviceDefinitions.getColorHelper(service.service_definition)}
					<div
						class="flex w-full min-w-0 items-center justify-center gap-1.5 py-1"
						title={service.name}
					>
						<ServiceIcon class="h-4 w-4 flex-shrink-0 {serviceColor.icon}" />
						<span class="text-secondary min-w-0 truncate">{service.name}</span>
					</div>
				{:else}
					<span class="text-tertiary">No services found</span>
				{/each}
			</div>
		</div>
	{:else if style === 'ImageBordered'}
		<div
			class="flex min-h-0 w-full flex-1 items-center justify-center overflow-hidden rounded-lg border-2 bg-white dark:bg-gray-800 {colorStyle.border}"
		>
			{#if data.imageUrl}
				<img src={data.imageUrl} alt="" class="h-full w-full object-cover" />
			{:else if IconComponent}
				<IconComponent class="h-8 w-8 {colorStyle.icon}" />
			{/if}
		</div>
	{:else}
		<!-- Image (default): no frame, just the glyph -->
		<div class="flex min-h-0 w-full flex-1 items-center justify-center overflow-hidden">
			{#if data.imageUrl}
				<img src={data.imageUrl} alt="" class="h-full w-full rounded object-cover" />
			{:else if IconComponent}
				<IconComponent class="h-10 w-10 {colorStyle.icon}" />
			{/if}
		</div>
	{/if}

	{#if style !== 'StatsCard'}
		<span
			class="object-label rounded bg-white/80 px-1 text-gray-700 dark:bg-gray-900/80 dark:text-gray-200"
			title={data.label}
		>
			{data.label}
		</span>
	{/if}
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
	.custom-object-node.selected {
		outline: 2px solid var(--color-primary, #3b82f6);
		outline-offset: 2px;
		border-radius: 0.5rem;
	}

	.object-label {
		display: -webkit-box;
		min-height: 0;
		max-height: 40%;
		max-width: 100%;
		flex-shrink: 1;
		overflow: hidden;
		overflow-wrap: anywhere;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 3;
		line-clamp: 3;
		line-height: 1.2;
		text-align: inherit;
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

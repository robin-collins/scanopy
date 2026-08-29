<script lang="ts">
	import { BaseEdge, EdgeLabel, getBezierPath, type EdgeProps } from '@xyflow/svelte';
	import type { components } from '$lib/api/schema';
	import { createColorHelper } from '$lib/shared/utils/styling';
	import { getFontCssStack } from './fonts';
	import { getTextFontSize } from './custom-view-model';

	interface CustomViewEdgeData extends Record<string, unknown> {
		fontFamily: string | null;
		fontSize: number;
		textColor: components['schemas']['Color'] | null;
		fontBold: boolean | null;
		fontItalic: boolean | null;
		fontUnderline: boolean | null;
		textAlign: components['schemas']['TextAlign'] | null;
	}

	let {
		id,
		interactionWidth,
		label,
		markerEnd,
		markerStart,
		sourcePosition,
		sourceX,
		sourceY,
		style,
		targetPosition,
		targetX,
		targetY,
		data
	}: EdgeProps & { data?: CustomViewEdgeData } = $props();

	let [path, labelX, labelY] = $derived(
		getBezierPath({ sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition })
	);
	let labelColor = $derived(createColorHelper(data?.textColor ?? 'Gray').rgb);
</script>

<BaseEdge {id} {path} {markerStart} {markerEnd} {interactionWidth} {style} />

{#if label}
	<EdgeLabel x={labelX} y={labelY} selectEdgeOnClick transparent>
		<div
			class="custom-view-edge-label nodrag nopan"
			data-edge-label-id={id}
			title={label}
			style:color={labelColor}
			style:font-family={getFontCssStack(data?.fontFamily)}
			style:font-size={`${getTextFontSize(data?.fontSize)}px`}
			style:font-weight={data?.fontBold ? '700' : '400'}
			style:font-style={data?.fontItalic ? 'italic' : 'normal'}
			style:text-decoration={data?.fontUnderline ? 'underline' : 'none'}
			style:text-align={(data?.textAlign ?? 'Left').toLowerCase()}
		>
			{label}
		</div>
	</EdgeLabel>
{/if}

<style>
	.custom-view-edge-label {
		box-sizing: border-box;
		width: max-content;
		max-width: 16rem;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		border-radius: 0.25rem;
		background: rgb(255 255 255 / 0.9);
		padding: 0.125rem 0.25rem;
		line-height: 1.25;
	}

	:global(.dark) .custom-view-edge-label {
		background: rgb(17 24 39 / 0.9);
	}
</style>

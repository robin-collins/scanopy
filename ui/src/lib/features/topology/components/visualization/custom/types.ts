/**
 * Node `data` shapes for the custom topology view canvas's three xyflow node
 * types (`object`, `text`, `group`) — see CustomViewCanvas.svelte for how
 * backend `CustomViewNode` records are converted into these.
 */

import type { CustomViewNode } from '$lib/features/custom-topology-views/queries';

export interface StatItem {
	label: string;
	value: string;
}

/** `kind = Entity | Library` — rendered by CustomObjectNode.svelte. */
export interface CustomObjectNodeData {
	view: CustomViewNode;
	/** Resolved display label (entity name, library object name, or the label override). */
	label: string;
	/** Resolved image URL: per-node upload > library object image > entity's own image, or null. */
	imageUrl: string | null;
	/** Kebab-case lucide icon name fallback when there's no image. */
	icon: string | null;
	/** Only populated for `StatsCard` style on a Host entity node. */
	stats?: StatItem[];
	[key: string]: unknown;
}

/** `kind = Text` — rendered by CustomTextNode.svelte. */
export interface CustomTextNodeData {
	view: CustomViewNode;
	onTextChange: (text: string) => void;
	[key: string]: unknown;
}

/** `kind = Group` — rendered by CustomGroupNode.svelte. */
export interface CustomGroupNodeData {
	view: CustomViewNode;
	onLabelChange: (label: string) => void;
	onResizeEnd: (width: number, height: number) => void;
	[key: string]: unknown;
}

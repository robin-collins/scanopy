import type { CanvasTypographyDefaults } from './custom-view-model';
/**
 * Node `data` shapes for the custom topology view canvas's three xyflow node
 * types (`object`, `text`, `customGroup`) — see CustomViewCanvas.svelte for how
 * backend `CustomViewNode` records are converted into these.
 */

import type { CustomViewNode } from '$lib/features/custom-topology-views/queries';
import type { Service } from '$lib/features/services/types/base';

export interface CanvasNodeBounds {
	x: number;
	y: number;
	width: number;
	height: number;
}

/** `kind = Entity | Library` — rendered by CustomObjectNode.svelte. */
export interface CustomObjectNodeData {
	view: CustomViewNode;
	/** Canvas-level typography this node inherits unless it overrides. */
	canvasDefaults: CanvasTypographyDefaults;
	/** Resolved display label (entity name, library object name, or the label override). */
	label: string;
	/** Resolved image URL: per-node upload > library object image > entity's own image, or null. */
	imageUrl: string | null;
	/** Kebab-case lucide icon name fallback when there's no image. */
	icon: string | null;
	/** Small text shown above the label on the `StatsCard` style (e.g. hostname/manufacturer). */
	headerText?: string | null;
	/** Only populated for `StatsCard` style on a Host entity node — the host's live services. */
	services?: Service[];
	onResizeEnd: (bounds: CanvasNodeBounds) => void;
	[key: string]: unknown;
}

/** `kind = Text` — rendered by CustomTextNode.svelte. */
export interface CustomTextNodeData {
	view: CustomViewNode;
	/** Canvas-level typography this node inherits unless it overrides. */
	canvasDefaults: CanvasTypographyDefaults;
	/** Select this node even when its editable surface consumes the pointer event. */
	onSelect: () => void;
	onTextChange: (text: string) => Promise<boolean>;
	onResizeEnd: (bounds: CanvasNodeBounds) => void;
	onAutoGrow: (bounds: CanvasNodeBounds) => void;
	[key: string]: unknown;
}

/** `kind = Group` — rendered by CustomGroupNode.svelte. */
export interface CustomGroupNodeData {
	view: CustomViewNode;
	/** Canvas-level typography this node inherits unless it overrides. */
	canvasDefaults: CanvasTypographyDefaults;
	onLabelChange: (label: string) => void;
	onResizeEnd: (bounds: CanvasNodeBounds) => void;
	[key: string]: unknown;
}

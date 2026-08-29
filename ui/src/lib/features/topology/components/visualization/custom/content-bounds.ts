import type { Node } from '@xyflow/svelte';

export interface Rect {
	x: number;
	y: number;
	width: number;
	height: number;
}

/**
 * Canvas content bounds for fit-view.
 *
 * SvelteFlow's own fitView measures nodes only, so anything rendered outside a
 * node box is invisible to it. Two confirmed requirements depend on that not
 * being true: connector labels must be reachable by fit-view, and free-text
 * nodes that auto-grew must be brought fully into view. Auto-grown nodes are
 * covered as long as their new dimensions are written back onto the node;
 * connector labels never are, because they live on the edge.
 */
export function unionRects(rects: readonly Rect[]): Rect | null {
	if (rects.length === 0) return null;

	let minX = Infinity;
	let minY = Infinity;
	let maxX = -Infinity;
	let maxY = -Infinity;

	for (const rect of rects) {
		minX = Math.min(minX, rect.x);
		minY = Math.min(minY, rect.y);
		maxX = Math.max(maxX, rect.x + rect.width);
		maxY = Math.max(maxY, rect.y + rect.height);
	}

	return { x: minX, y: minY, width: maxX - minX, height: maxY - minY };
}

/**
 * A node's occupied rectangle. Prefers the measured size, which is where an
 * auto-grown free-text node's real height ends up, and falls back to the
 * declared width/height before the first measurement pass.
 */
export function getNodeRect(node: Node): Rect | null {
	const width = node.measured?.width ?? node.width;
	const height = node.measured?.height ?? node.height;

	if (typeof width !== 'number' || typeof height !== 'number') return null;
	if (width <= 0 || height <= 0) return null;

	return { x: node.position.x, y: node.position.y, width, height };
}

export function getCanvasContentBounds(
	nodes: readonly Node[],
	labelRects: readonly Rect[] = []
): Rect | null {
	const rects: Rect[] = [];

	for (const node of nodes) {
		const rect = getNodeRect(node);
		if (rect) rects.push(rect);
	}

	rects.push(...labelRects);
	return unionRects(rects);
}

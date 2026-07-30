import type { TopologyView } from '../queries';
import type { TopologyLayoutOverride, TopologyNode } from '../types/base';
import type { LayoutGraph } from './layout-graph';

export const LAYOUT_GRID_SIZE = 25;

export function snapPositionToGrid(
	position: { x: number; y: number },
	gridSize = LAYOUT_GRID_SIZE
): { x: number; y: number } {
	return {
		x: Math.round(position.x / gridSize) * gridSize,
		y: Math.round(position.y / gridSize) * gridSize
	};
}

/** Only leaf cards and L2 host containers are manually positionable. */
export function canManuallyPositionNode(node: TopologyNode): boolean {
	return (
		node.node_type === 'Element' ||
		(node.node_type === 'Container' && node.container_type === 'Host')
	);
}

/**
 * Apply saved positions after automatic layout. Stale overrides are deliberately
 * ignored when their node disappeared or changed immediate parent.
 */
export function applyLayoutOverrides(
	graph: LayoutGraph,
	overrides: readonly TopologyLayoutOverride[],
	view: TopologyView
): number {
	let applied = 0;
	for (const override of overrides) {
		if (override.view !== view || !graph.hasNode(override.node_id)) continue;
		if (graph.getParentId(override.node_id) !== override.parent_node_id) continue;
		if (graph.setPosition(override.node_id, override.position)) applied += 1;
	}
	return applied;
}

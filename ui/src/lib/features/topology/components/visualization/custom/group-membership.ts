import type { NodeKind } from '$lib/features/custom-topology-views/queries';

export type CompletedBoundsChangeCause =
	| 'node-drag'
	| 'node-resize'
	| 'group-drag'
	| 'group-resize';

/** Final rendered geometry in absolute canvas coordinates. */
export interface CanvasNodeGeometry {
	id: string;
	kind: NodeKind;
	parentNodeId: string | null;
	x: number;
	y: number;
	width: number;
	height: number;
}

/** Coordinates to persist; parented coordinates are relative to the returned parent. */
export interface MembershipPatch {
	id: string;
	parentNodeId: string | null;
	x: number;
	y: number;
}

function containsCenter(group: CanvasNodeGeometry, node: CanvasNodeGeometry): boolean {
	const centerX = node.x + node.width / 2;
	const centerY = node.y + node.height / 2;
	return (
		centerX >= group.x &&
		centerX <= group.x + group.width &&
		centerY >= group.y &&
		centerY <= group.y + group.height
	);
}

function enclosingGroups(
	nodes: CanvasNodeGeometry[],
	node: CanvasNodeGeometry
): CanvasNodeGeometry[] {
	return nodes
		.filter((candidate) => candidate.kind === 'Group' && candidate.id !== node.id)
		.filter((group) => containsCenter(group, node))
		.sort((a, b) => {
			const areaDifference = a.width * a.height - b.width * b.height;
			return areaDifference === 0 ? a.id.localeCompare(b.id) : areaDifference;
		});
}

function relativePatch(
	node: CanvasNodeGeometry,
	parent: CanvasNodeGeometry | null
): MembershipPatch {
	return {
		id: node.id,
		parentNodeId: parent?.id ?? null,
		x: Math.round(parent ? node.x - parent.x : node.x),
		y: Math.round(parent ? node.y - parent.y : node.y)
	};
}

/**
 * Reconcile membership exactly once after a rendered bounds change completes.
 *
 * The cause is deliberately explicit so pointer drags, resizers, and future
 * programmatic auto-growth all enter through the same rule set without
 * continuously capturing/releasing nodes while their bounds are changing.
 */
export function reconcileCompletedBoundsChange(
	nodes: CanvasNodeGeometry[],
	changedNodeId: string,
	cause: CompletedBoundsChangeCause
): MembershipPatch[] {
	const changed = nodes.find((node) => node.id === changedNodeId);
	if (!changed) return [];

	if (cause === 'node-drag' || cause === 'node-resize') {
		if (changed.kind === 'Group') return [];

		const currentParent = changed.parentNodeId
			? (nodes.find((node) => node.id === changed.parentNodeId && node.kind === 'Group') ?? null)
			: null;

		if (cause === 'node-resize' && currentParent) {
			// Resizing may detach a child whose centre left its frame, but it
			// cannot implicitly reparent that existing child into another frame.
			return [
				relativePatch(changed, containsCenter(currentParent, changed) ? currentParent : null)
			];
		}

		const parent = enclosingGroups(nodes, changed)[0] ?? null;
		return [relativePatch(changed, parent)];
	}

	if (changed.kind !== 'Group') return [];

	const patches: MembershipPatch[] = [];
	for (const node of nodes) {
		if (node.kind === 'Group' || node.id === changed.id) continue;

		if (node.parentNodeId === null && containsCenter(changed, node)) {
			patches.push(relativePatch(node, changed));
			continue;
		}

		if (
			cause === 'group-resize' &&
			node.parentNodeId === changed.id &&
			!containsCenter(changed, node)
		) {
			patches.push(relativePatch(node, null));
		}
	}

	return patches;
}

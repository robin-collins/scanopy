import { describe, expect, it, vi } from 'vitest';
import { writable } from 'svelte/store';
import type { Node, Edge } from '@xyflow/svelte';
import type { TopologyNode, TopologyLayoutOverride } from '$lib/features/topology/types/base';
import { LayoutGraph } from '$lib/features/topology/layout/layout-graph';
import {
	applyLayoutOverrides,
	canManuallyPositionNode,
	snapPositionToGrid
} from '$lib/features/topology/layout/layout-overrides';
import { createTopologyKeydownHandler } from '$lib/features/topology/keyboard';
import { resetNodePositions, updateNodePosition } from '$lib/features/topology/queries';

const apiMocks = vi.hoisted(() => ({
	POST: vi.fn(),
	DELETE: vi.fn()
}));

vi.mock('$lib/api/client', () => ({ apiClient: apiMocks }));

const container = (
	id: string,
	container_type: string,
	parent_container_id?: string
): TopologyNode =>
	({
		id,
		node_type: 'Container',
		container_type,
		...(parent_container_id ? { parent_container_id } : {})
	}) as unknown as TopologyNode;

const element = (id: string, container_id: string): TopologyNode =>
	({
		id,
		node_type: 'Element',
		container_id,
		host_id: 'host',
		element_type: 'Service'
	}) as unknown as TopologyNode;

const override = (
	node_id: string,
	parent_node_id: string | null,
	view: TopologyLayoutOverride['view'],
	position: { x: number; y: number }
): TopologyLayoutOverride => ({
	topology_id: 'topology',
	view,
	node_id,
	parent_node_id,
	position,
	created_at: '2026-07-19T00:00:00Z',
	updated_at: '2026-07-19T00:00:00Z'
});

describe('manual topology layout overrides', () => {
	it('applies only overrides for the active view with the current parent', () => {
		const graph = LayoutGraph.fromTopology([
			container('root', 'Subnet'),
			container('other', 'Subnet'),
			element('service', 'root')
		]);

		const applied = applyLayoutOverrides(
			graph,
			[
				override('service', 'root', 'L3Logical', { x: 125, y: 75 }),
				override('root', null, 'L3Logical', { x: 300, y: 200 }),
				override('other', null, 'L2Physical', { x: 900, y: 900 }),
				override('missing', null, 'L3Logical', { x: 1, y: 1 })
			],
			'L3Logical'
		);

		expect(applied).toBe(2);
		expect(graph.getPosition('service')).toEqual({ x: 125, y: 75 });
		expect(graph.getPosition('root')).toEqual({ x: 300, y: 200 });
		expect(graph.getPosition('other')).toEqual({ x: 0, y: 0 });
	});

	it('ignores a saved position when the node moved to another parent', () => {
		const graph = LayoutGraph.fromTopology([
			container('old-parent', 'Subnet'),
			container('new-parent', 'Subnet'),
			element('service', 'new-parent')
		]);

		expect(
			applyLayoutOverrides(
				graph,
				[override('service', 'old-parent', 'L3Logical', { x: 500, y: 500 })],
				'L3Logical'
			)
		).toBe(0);
		expect(graph.getPosition('service')).toEqual({ x: 0, y: 0 });
	});

	it('snaps drag positions and limits dragging to elements and Host containers', () => {
		expect(snapPositionToGrid({ x: 38, y: -13 })).toEqual({ x: 50, y: -25 });
		expect(canManuallyPositionNode(element('service', 'root'))).toBe(true);
		expect(canManuallyPositionNode(container('host', 'Host'))).toBe(true);
		expect(canManuallyPositionNode(container('subnet', 'Subnet'))).toBe(false);
	});
});

describe('topology edit shortcut', () => {
	it('uses E to toggle manual layout editing', () => {
		const onToggleEdit = vi.fn();
		const handler = createTopologyKeydownHandler({
			getBaseViewer: () => null,
			getShortcutsHelpOpen: () => false,
			setShortcutsHelpOpen: vi.fn(),
			selectionStores: {
				selectedNode: writable<Node | null>(null),
				selectedEdge: writable<Edge | null>(null),
				selectedNodes: writable<Node[]>([])
			},
			onToggleEdit
		});

		handler({
			key: 'E',
			target: null,
			metaKey: false,
			ctrlKey: false,
			altKey: false
		} as KeyboardEvent);

		expect(onToggleEdit).toHaveBeenCalledOnce();
	});
});

describe('layout mutation helpers', () => {
	it('sends the scoped position payload and view reset path', async () => {
		apiMocks.POST.mockResolvedValueOnce({ data: { success: true } });
		apiMocks.DELETE.mockResolvedValueOnce({ data: { success: true } });

		await updateNodePosition({
			topologyId: 'topology',
			view: 'L3Logical',
			nodeId: 'service',
			position: { x: 25, y: 50 }
		});
		await resetNodePositions({ topologyId: 'topology', view: 'L3Logical' });

		expect(apiMocks.POST).toHaveBeenCalledWith('/api/v1/topology/{id}/node-position', {
			params: { path: { id: 'topology' } },
			body: { view: 'L3Logical', node_id: 'service', position: { x: 25, y: 50 } }
		});
		expect(apiMocks.DELETE).toHaveBeenCalledWith('/api/v1/topology/{id}/node-positions/{view}', {
			params: { path: { id: 'topology', view: 'L3Logical' } }
		});
	});
});

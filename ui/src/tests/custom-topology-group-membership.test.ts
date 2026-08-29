import { describe, expect, it } from 'vitest';
import {
	reconcileCompletedBoundsChange,
	type CanvasNodeGeometry
} from '$lib/features/topology/components/visualization/custom/group-membership';

function node(overrides: Partial<CanvasNodeGeometry> & Pick<CanvasNodeGeometry, 'id'>) {
	return {
		kind: 'Entity',
		parentNodeId: null,
		x: 0,
		y: 0,
		width: 20,
		height: 20,
		...overrides
	} as CanvasNodeGeometry;
}

describe('custom topology group membership', () => {
	it('captures a partially overlapping node whose centre finishes on the frame boundary', () => {
		const nodes = [
			node({ id: 'group', kind: 'Group', width: 100, height: 100 }),
			node({ id: 'object', x: 90, y: 40 })
		];

		expect(reconcileCompletedBoundsChange(nodes, 'object', 'node-drag')).toEqual([
			{ id: 'object', parentNodeId: 'group', x: 90, y: 40 }
		]);
	});

	it('gives the smallest enclosing frame precedence after an explicit node drag', () => {
		const nodes = [
			node({ id: 'large', kind: 'Group', width: 300, height: 300 }),
			node({ id: 'small', kind: 'Group', x: 50, y: 50, width: 100, height: 100 }),
			node({ id: 'object', x: 80, y: 90 })
		];

		expect(reconcileCompletedBoundsChange(nodes, 'object', 'node-drag')).toEqual([
			{ id: 'object', parentNodeId: 'small', x: 30, y: 40 }
		]);
	});

	it('captures ungrouped nodes after a frame move without stealing another frame child', () => {
		const nodes = [
			node({ id: 'moved', kind: 'Group', x: 100, y: 100, width: 200, height: 200 }),
			node({ id: 'other', kind: 'Group', x: 50, y: 50, width: 300, height: 300 }),
			node({ id: 'free', x: 130, y: 140 }),
			node({ id: 'owned', parentNodeId: 'other', x: 150, y: 160 })
		];

		expect(reconcileCompletedBoundsChange(nodes, 'moved', 'group-drag')).toEqual([
			{ id: 'free', parentNodeId: 'moved', x: 30, y: 40 }
		]);
	});

	it('captures on frame growth and releases on frame shrink without visual jumps', () => {
		const grown = [
			node({ id: 'group', kind: 'Group', x: 100, y: 80, width: 250, height: 200 }),
			node({ id: 'free', x: 300, y: 150 })
		];
		expect(reconcileCompletedBoundsChange(grown, 'group', 'group-resize')).toEqual([
			{ id: 'free', parentNodeId: 'group', x: 200, y: 70 }
		]);

		const shrunk = [
			node({ id: 'group', kind: 'Group', x: 100, y: 80, width: 100, height: 100 }),
			node({ id: 'child', parentNodeId: 'group', x: 220, y: 150 })
		];
		expect(reconcileCompletedBoundsChange(shrunk, 'group', 'group-resize')).toEqual([
			{ id: 'child', parentNodeId: null, x: 220, y: 150 }
		]);
	});

	it('detaches or reparents an explicitly dragged child while preserving its absolute position', () => {
		const detached = [
			node({ id: 'old', kind: 'Group', width: 100, height: 100 }),
			node({ id: 'child', parentNodeId: 'old', x: 180, y: 120 })
		];
		expect(reconcileCompletedBoundsChange(detached, 'child', 'node-drag')).toEqual([
			{ id: 'child', parentNodeId: null, x: 180, y: 120 }
		]);

		const reparented = [
			node({ id: 'old', kind: 'Group', width: 100, height: 100 }),
			node({ id: 'new', kind: 'Group', x: 150, y: 100, width: 100, height: 100 }),
			node({ id: 'child', parentNodeId: 'old', x: 180, y: 120 })
		];
		expect(reconcileCompletedBoundsChange(reparented, 'child', 'node-drag')).toEqual([
			{ id: 'child', parentNodeId: 'new', x: 30, y: 20 }
		]);
	});

	it('preserves child-relative offsets during a frame move', () => {
		const nodes = [
			node({ id: 'group', kind: 'Group', x: 200, y: 150, width: 200, height: 200 }),
			node({ id: 'child', parentNodeId: 'group', x: 235, y: 180 })
		];

		expect(reconcileCompletedBoundsChange(nodes, 'group', 'group-drag')).toEqual([]);
	});

	it('lets a completed node resize detach but never implicitly reparent an existing child', () => {
		const nodes = [
			node({ id: 'old', kind: 'Group', width: 100, height: 100 }),
			node({ id: 'new', kind: 'Group', x: 150, y: 100, width: 100, height: 100 }),
			node({ id: 'child', parentNodeId: 'old', x: 180, y: 120, width: 40, height: 40 })
		];

		expect(reconcileCompletedBoundsChange(nodes, 'child', 'node-resize')).toEqual([
			{ id: 'child', parentNodeId: null, x: 180, y: 120 }
		]);
	});
});

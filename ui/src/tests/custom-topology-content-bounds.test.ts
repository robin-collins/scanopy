import { describe, it, expect } from 'vitest';
import type { Node } from '@xyflow/svelte';
import {
	getCanvasContentBounds,
	getNodeRect,
	unionRects
} from '$lib/features/topology/components/visualization/custom/content-bounds';

const node = (id: string, x: number, y: number, width: number, height: number): Node =>
	({ id, position: { x, y }, data: {}, measured: { width, height } }) as Node;

describe('canvas content bounds', () => {
	it('is null when there is nothing to bound', () => {
		expect(getCanvasContentBounds([], [])).toBeNull();
		expect(unionRects([])).toBeNull();
	});

	it('spans every node', () => {
		const bounds = getCanvasContentBounds([node('a', 0, 0, 100, 50), node('b', 200, 120, 40, 40)]);
		expect(bounds).toEqual({ x: 0, y: 0, width: 240, height: 160 });
	});

	it('extends to a connector label lying outside every node', () => {
		const nodes = [node('a', 0, 0, 100, 100)];
		const withoutLabel = getCanvasContentBounds(nodes, []);
		const withLabel = getCanvasContentBounds(nodes, [{ x: 300, y: -60, width: 120, height: 20 }]);

		expect(withoutLabel).toEqual({ x: 0, y: 0, width: 100, height: 100 });
		expect(withLabel).toEqual({ x: 0, y: -60, width: 420, height: 160 });
	});

	it('prefers a measured size, which is where auto-growth lands', () => {
		const grown = {
			id: 'text',
			position: { x: 10, y: 10 },
			data: {},
			width: 120,
			height: 40,
			measured: { width: 120, height: 300 }
		} as Node;
		expect(getNodeRect(grown)).toEqual({ x: 10, y: 10, width: 120, height: 300 });
	});

	it('ignores a node with no usable size rather than collapsing the bounds', () => {
		const unmeasured = { id: 'x', position: { x: 5, y: 5 }, data: {} } as Node;
		expect(getNodeRect(unmeasured)).toBeNull();
		expect(getCanvasContentBounds([unmeasured, node('a', 0, 0, 10, 10)])).toEqual({
			x: 0,
			y: 0,
			width: 10,
			height: 10
		});
	});
});

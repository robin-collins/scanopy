import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { getAutoGrowBounds } from '$lib/features/topology/components/visualization/custom/text-overflow';

const component = (name: string) =>
	readFileSync(
		resolve(__dirname, `../lib/features/topology/components/visualization/custom/${name}.svelte`),
		'utf-8'
	);

describe('free-text auto growth', () => {
	it('grows height to the rendered content without shrinking the selected width', () => {
		expect(
			getAutoGrowBounds(
				{ currentWidth: 180, currentHeight: 80, contentWidth: 180, contentHeight: 143.2 },
				{ x: 25, y: 35 }
			)
		).toEqual({ x: 25, y: 35, width: 180, height: 144 });
	});

	it('widens for unbreakable rendered content and never returns a shrink', () => {
		expect(
			getAutoGrowBounds(
				{ currentWidth: 100, currentHeight: 60, contentWidth: 130.1, contentHeight: 80.1 },
				{ x: 0, y: 0 }
			)
		).toEqual({ x: 0, y: 0, width: 131, height: 81 });
		expect(
			getAutoGrowBounds(
				{ currentWidth: 180, currentHeight: 80, contentWidth: 120, contentHeight: 40 },
				{ x: 0, y: 0 }
			)
		).toBeNull();
	});

	it('persists through the shared completed-bounds path with a distinct cause', () => {
		const canvas = component('CustomViewCanvas');
		expect(canvas).toContain("'text-auto-resize'");
		expect(canvas).toContain('saveCompletedBoundsChange(');
		expect(canvas).toContain('onAutoGrow: (bounds) => handleTextAutoGrow(view, bounds)');
	});
});

describe('bounded custom-canvas text', () => {
	it('wraps and clamps fixed object and group labels without mutating their values', () => {
		const objectNode = component('CustomObjectNode');
		const groupNode = component('CustomGroupNode');
		expect(objectNode).toContain('overflow-wrap: anywhere');
		expect(objectNode).toContain('-webkit-line-clamp: 3');
		expect(groupNode).toContain('overflow-wrap: anywhere');
		expect(groupNode).toContain('-webkit-line-clamp: 3');
		expect(groupNode).toContain('group-description min-h-0 flex-1');
		expect(groupNode).not.toContain('truncate text-xs');
	});

	it('keeps card rows in an internal scrollport and exposes full headings as titles', () => {
		const objectNode = component('CustomObjectNode');
		expect(objectNode).toContain('overflow-y-auto overscroll-contain');
		expect(objectNode).toContain('title={data.headerText}');
		expect(objectNode).toContain('title={data.label}');
	});

	it('uses a wrapping HTML connector label instead of the non-wrapping built-in label', () => {
		const edge = component('CustomViewEdge');
		expect(edge).toContain('white-space: pre-wrap');
		expect(edge).toContain('overflow-wrap: anywhere');
		expect(edge).not.toContain('text-overflow: ellipsis');
	});
});

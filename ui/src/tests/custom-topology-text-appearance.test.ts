import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const inspector = readFileSync(
	new URL(
		'../lib/features/topology/components/visualization/custom/CustomViewNodeInspector.svelte',
		import.meta.url
	),
	'utf8'
);
const canvasPanel = readFileSync(
	new URL(
		'../lib/features/topology/components/visualization/custom/CanvasControlPanel.svelte',
		import.meta.url
	),
	'utf8'
);
const canvas = readFileSync(
	new URL(
		'../lib/features/topology/components/visualization/custom/CustomViewCanvas.svelte',
		import.meta.url
	),
	'utf8'
);
const nodeRenderers = [
	'CustomObjectNode.svelte',
	'CustomTextNode.svelte',
	'CustomGroupNode.svelte'
].map((file) =>
	readFileSync(
		new URL(`../lib/features/topology/components/visualization/custom/${file}`, import.meta.url),
		'utf8'
	)
);
const edgeRenderer = readFileSync(
	new URL(
		'../lib/features/topology/components/visualization/custom/CustomViewEdge.svelte',
		import.meta.url
	),
	'utf8'
);

describe('custom topology text appearance controls', () => {
	it('allows every node override to be cleared back to its canvas value', () => {
		expect(inspector).toContain('onUpdate({ text_color: null })');
		expect(inspector).toContain("return value === '' ? null : value === 'true'");
		expect(inspector).toContain("value={node.text_align ?? ''}");
		expect(inspector).toContain('(event.target as HTMLSelectElement).value || null');
	});

	it('exposes independent nullable canvas defaults', () => {
		expect(canvasPanel).toContain('onUpdate({ default_text_color: null })');
		for (const field of [
			'default_font_bold',
			'default_font_italic',
			'default_font_underline',
			'default_text_align'
		]) {
			expect(canvasPanel).toContain(field);
		}
	});

	it('uses the text-colour default rather than decorative colour for connector labels', () => {
		expect(canvas).toContain('textColor: currentView?.default_text_color ?? null');
		expect(canvas).not.toContain('textColor: currentView?.default_primary_color');
	});

	it('applies resolved text colour to every node category and canvas emphasis to connectors', () => {
		for (const renderer of nodeRenderers) {
			expect(renderer).toContain('style:color={appearance.textColor}');
			expect(renderer).not.toContain('style:color={appearance.primary}');
		}
		expect(edgeRenderer).toContain("style:font-weight={data?.fontBold ? '700' : '400'}");
		expect(edgeRenderer).toContain("style:text-align={(data?.textAlign ?? 'Left').toLowerCase()}");
	});

	it('accepts a canvas default font size above the removed 72px ceiling', () => {
		// Regression: the node-level ceiling was removed in T-21 but CanvasControlPanel's
		// own handler kept a hardcoded `value <= 72`, so a canvas default above 72 was
		// silently dropped -- the input accepted it and the save never happened.
		expect(canvasPanel).not.toMatch(/default_font_size[\s\S]{0,200}?<=\s*72/);
		expect(canvasPanel).toContain('Number.isSafeInteger(value) && value >= 10');
	});
});

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
});

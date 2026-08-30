import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const readCustomComponent = (file: string) =>
	readFileSync(
		new URL(`../lib/features/topology/components/visualization/custom/${file}`, import.meta.url),
		'utf8'
	);

const canvas = readCustomComponent('CustomViewCanvas.svelte');
const inspector = readCustomComponent('CustomViewNodeInspector.svelte');
const textNode = readCustomComponent('CustomTextNode.svelte');

describe('custom topology text properties', () => {
	it('selects an editable text surface and resolves it through the shared node inspector', () => {
		expect(textNode).toContain('data.onSelect();');
		expect(textNode).toContain('onpointerdown={selectForEditing}');
		expect(canvas).toContain('selectedNodeId = view.id;');
		expect(canvas).toContain('{#if selectedNode}');
		expect(canvas).toContain('<CustomViewNodeInspector');
	});

	it('exposes the rendered text body as a bounded Content textarea', () => {
		expect(inspector).toContain("{#if node.kind === 'Text'}");
		expect(inspector).toContain('Content');
		expect(inspector).toContain('maxlength={5000}');
		expect(inspector).toContain('onUpdate({ text_content: textContentDraft });');
	});

	it('synchronizes panel and inline edits without clobbering either active draft', () => {
		expect(inspector).toContain("focusedMetadataDraft !== 'text_content'");
		expect(textNode).toContain('else if (!editing && text !== nextText)');
		expect(textNode).toContain('text = nextText;');
	});
});

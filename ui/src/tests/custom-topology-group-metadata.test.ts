import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const inspector = readFileSync(
	resolve(
		__dirname,
		'../lib/features/topology/components/visualization/custom/CustomViewNodeInspector.svelte'
	),
	'utf-8'
);
const groupNode = readFileSync(
	resolve(
		__dirname,
		'../lib/features/topology/components/visualization/custom/CustomGroupNode.svelte'
	),
	'utf-8'
);

describe('custom topology group metadata', () => {
	it('bounds every editable group text field at the server-supported limit', () => {
		expect(inspector.match(/maxlength=\{200\}/g)?.length).toBeGreaterThanOrEqual(2);
		expect(inspector).toContain('maxlength={2000}');
		expect(groupNode).toContain('maxlength={200}');
	});

	it('persists visibility independently without clearing either stored text field', () => {
		expect(inspector).toContain('onUpdate({ show_label:');
		expect(inspector).toContain('onUpdate({ show_description:');
		expect(inspector).not.toMatch(/show_label[^\n]+(?:label|description):\s*(?:null|'')/);
		expect(inspector).not.toMatch(/show_description[^\n]+(?:label|description):\s*(?:null|'')/);
	});

	it('keeps hidden text in the record and gates only its rendered region', () => {
		expect(groupNode).toContain('data.view.show_label !== false');
		expect(groupNode).toContain('data.view.show_description !== false && data.view.description');
	});

	it('synchronizes successful same-node label updates without clobbering an active draft', () => {
		expect(inspector).toContain("focusedMetadataDraft !== 'label'");
		expect(groupNode).toContain('nextLabel !== labelSource');
		expect(groupNode).toContain('if (!editingLabel) label = nextLabel');
	});
});

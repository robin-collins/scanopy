import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const read = (file: string) =>
	readFileSync(
		new URL(`../lib/features/topology/components/visualization/custom/${file}`, import.meta.url),
		'utf8'
	);

const canvas = read('CustomViewCanvas.svelte');
const node = read('CustomObjectNode.svelte');
const inspector = read('CustomViewNodeInspector.svelte');

describe('custom topology service presentation', () => {
	it('reuses the same metadata icon resolver as the L3 Physical topology', () => {
		expect(node).toContain("import { serviceDefinitions } from '$lib/shared/stores/metadata'");
		expect(node).toContain('serviceDefinitions.getIconComponent(data.serviceDefinition)');
		expect(canvas).toContain('serviceDefinition: service?.service_definition ?? null');
	});

	it('offers icon visibility, detected-default reset, and all required positions', () => {
		expect(inspector).toContain('Show service icon');
		expect(inspector).toContain('Before name');
		expect(inspector).toContain('After name');
		expect(inspector).toContain('Centre of object');
		expect(inspector).toContain('Custom icon URL');
		expect(inspector).toContain('onUpdate({ service_icon_url: null })');
	});

	it('keeps label anchoring and offsets independent', () => {
		for (const label of ['Horizontal', 'Vertical', 'X offset', 'Y offset']) {
			expect(inspector).toContain(label);
		}
		expect(node).toContain('style:justify-content={serviceLabelPlacement.justifyContent}');
		expect(node).toContain('style:align-items={serviceLabelPlacement.alignItems}');
		expect(node).toContain('style:transform={serviceLabelPlacement.transform}');
	});
});

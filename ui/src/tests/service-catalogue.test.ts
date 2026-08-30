import { beforeEach, describe, expect, it } from 'vitest';
import { get } from 'svelte/store';
import { metadata, serviceDefinitions, type TypeMetadata } from '$lib/shared/stores/metadata';
import { applyCustomServiceDefinitions } from '$lib/features/services/service-catalogue';
import type { ServiceCatalogueEntry } from '$lib/features/services/service-catalogue';

const customEntry: ServiceCatalogueEntry = {
	kind: 'custom',
	id: 'My Custom Service',
	name: 'My Custom Service',
	description: 'A user-created service',
	category: 'Database',
	color: 'Blue',
	icon: 'database',
	logo_url: '',
	logo_needs_white_background: false,
	is_generic: false,
	custom_id: '0f2b3e80-0000-4000-8000-000000000000'
};

describe('service catalogue metadata merge', () => {
	let baseline: TypeMetadata[];

	beforeEach(() => {
		// Reset any custom entries left behind by an earlier test run so the
		// merge assertions start from a clean built-in-only registry, then
		// capture that clean baseline.
		metadata.update(($m) => ({
			...$m,
			service_definitions: $m.service_definitions.filter((item) => item.id !== customEntry.id)
		}));
		baseline = get(metadata).service_definitions;
	});

	it('appends custom entries to the metadata registry', () => {
		applyCustomServiceDefinitions([customEntry]);

		const item = serviceDefinitions.getItem(customEntry.id);
		expect(item).not.toBeNull();
		expect(item?.name).toBe(customEntry.name);
		expect(item?.category).toBe(customEntry.category);
		expect(item?.metadata).toMatchObject({
			can_be_added: true,
			is_generic: false,
			custom_service_definition_id: customEntry.custom_id
		});
	});

	it('leaves built-in entries untouched', () => {
		const before = baseline;
		applyCustomServiceDefinitions([customEntry]);
		const after = get(metadata).service_definitions;
		expect(after.length).toBe(before.length + 1);
		for (const builtIn of before) {
			expect(after.some((item) => item.id === builtIn.id)).toBe(true);
		}
	});

	it('is idempotent across repeated loads', () => {
		applyCustomServiceDefinitions([customEntry]);
		applyCustomServiceDefinitions([customEntry]);
		const after = get(metadata).service_definitions;
		expect(after.filter((item) => item.id === customEntry.id)).toHaveLength(1);
	});
});

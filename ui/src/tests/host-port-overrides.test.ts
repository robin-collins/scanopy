import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import {
	buildServiceReferenceOptions,
	decodeServiceReference,
	encodeServiceReference,
	referenceForCatalogueEntry
} from '$lib/features/host-port-overrides/service-reference';
import type { ServiceCatalogueEntry } from '$lib/features/services/service-catalogue';

const read = (relativePath: string) =>
	readFileSync(new URL(`../lib/${relativePath}`, import.meta.url), 'utf8');

const portsForm = read('features/hosts/components/HostEditModal/Ports/PortsForm.svelte');
const panel = read('features/host-port-overrides/components/PortOverridePanel.svelte');
const queries = read('features/host-port-overrides/queries.ts');

const builtIn: ServiceCatalogueEntry = {
	kind: 'built_in',
	id: 'HTTP Server',
	name: 'HTTP Server',
	description: '',
	category: 'NetworkAccess',
	color: null,
	icon: null,
	logo_url: '',
	logo_needs_white_background: false,
	is_generic: true,
	custom_id: null
};

const custom: ServiceCatalogueEntry = {
	...builtIn,
	kind: 'custom',
	id: 'Internal Dashboard',
	name: 'Internal Dashboard',
	custom_id: '0f2b3e80-0000-4000-8000-000000000000'
};

describe('per-host port overrides', () => {
	it('mounts the override editor from the host ports form', () => {
		expect(portsForm).toContain(
			"import PortOverridePanel from '$lib/features/host-port-overrides/components/PortOverridePanel.svelte'"
		);
		expect(portsForm).toContain('<PortOverridePanel port={selectedItem} />');
		expect(portsForm).toMatch(
			/type == 'Custom'[\s\S]*<PortConfigPanel[\s\S]*<PortOverridePanel port=\{selectedItem\}/
		);
	});

	it('selects an override by its stable host, number, and protocol value key', () => {
		expect(panel).toContain('let hostId = $derived(port.host_id)');
		expect(panel).toContain('o.port_number === port.number && o.port_protocol === port.protocol');
		expect(queries).toContain('queryKey: queryKeys.hostPortOverrides.byHost(hostId())');
	});

	it('persists trimmed display fields and represents fallback with null', () => {
		expect(panel).toContain('display_name: trimmedName.length > 0 ? trimmedName : null');
		expect(panel).toContain('icon_url: trimmedIcon.length > 0 ? trimmedIcon : null');
		expect(panel).toContain("displayName = override?.display_name ?? ''");
		expect(panel).toContain("iconUrl = override?.icon_url ?? ''");
	});

	it('supports clearing the tuple override back to global defaults', () => {
		expect(panel).toContain('useClearHostPortOverrideMutation');
		expect(panel).toContain('await clearMutation.mutateAsync({');
		expect(panel).toContain('port_number: port.number');
		expect(panel).toContain('port_protocol: port.protocol');
		expect(queries).toContain(
			"'/api/v1/host-port-overrides/{host_id}/{port_number}/{port_protocol}'"
		);
	});

	it('maps merged catalogue namespaces to the tagged-union reference shape', () => {
		expect(referenceForCatalogueEntry(builtIn)).toEqual({ kind: 'BuiltIn', id: 'HTTP Server' });
		expect(referenceForCatalogueEntry(custom)).toEqual({
			kind: 'Custom',
			id: custom.custom_id
		});
		const encoded = encodeServiceReference({ kind: 'Custom', id: custom.custom_id! });
		expect(decodeServiceReference(encoded)).toEqual({ kind: 'Custom', id: custom.custom_id });
		expect(decodeServiceReference('not-json')).toBeNull();
	});

	it('keeps a dangling service reference visible as its raw id', () => {
		const dangling = { kind: 'Custom' as const, id: 'deadbeef-0000-4000-8000-000000000000' };
		const options = buildServiceReferenceOptions([builtIn, custom], dangling, 'Unknown');
		expect(options).toContainEqual({
			value: encodeServiceReference(dangling),
			label: `Unknown: ${dangling.id}`
		});
	});

	it('submits the selected service reference instead of clearing it', () => {
		expect(panel).toContain('useServiceCatalogueQuery');
		expect(panel).toContain('service_ref_kind: serviceRefKind');
		expect(panel).toContain('service_ref_id: serviceRefKind ? serviceRefId : null');
	});
});

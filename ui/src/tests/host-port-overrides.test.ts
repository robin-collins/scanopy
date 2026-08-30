import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const read = (relativePath: string) =>
	readFileSync(new URL(`../lib/${relativePath}`, import.meta.url), 'utf8');

const portsForm = read('features/hosts/components/HostEditModal/Ports/PortsForm.svelte');
const panel = read('features/host-port-overrides/components/PortOverridePanel.svelte');
const queries = read('features/host-port-overrides/queries.ts');

describe('per-host port overrides', () => {
	it('mounts the override editor from the host ports form', () => {
		expect(portsForm).toContain(
			"import PortOverridePanel from '$lib/features/host-port-overrides/components/PortOverridePanel.svelte'"
		);
		expect(portsForm).toContain('<PortOverridePanel port={selectedItem} />');
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
});

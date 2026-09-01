import { beforeEach, describe, expect, it } from 'vitest';
import { get } from 'svelte/store';
import { metadata, ports } from '$lib/shared/stores/metadata';
import {
	applyCustomKnownPorts,
	findCustomKnownPort,
	type PortTypeEntry
} from '$lib/features/known_ports/catalogue';
import type { KnownPort } from '$lib/features/known_ports/types';

const builtIn: KnownPort = {
	id: 'Ssh',
	organization_id: null,
	source: 'BuiltIn',
	name: 'SSH',
	description: 'Secure Shell',
	port_number: 22,
	transport_protocol: 'Tcp'
};

const custom: KnownPort = {
	id: '3b1d4c2e-0000-4000-8000-000000000000',
	organization_id: '9e8f7a6b-0000-4000-8000-000000000000',
	source: 'Custom',
	name: 'Internal Dashboard',
	description: 'Ops dashboard',
	port_number: 17443,
	transport_protocol: 'Tcp'
};

describe('known ports metadata merge', () => {
	let builtInCount: number;

	beforeEach(() => {
		applyCustomKnownPorts([]);
		builtInCount = get(metadata).ports.length;
	});

	it('projects only the custom layer onto the ports registry', () => {
		applyCustomKnownPorts([builtIn, custom]);

		expect(get(metadata).ports).toHaveLength(builtInCount + 1);
		const entry = ports.getItem(custom.id);
		expect(entry?.name).toBe('Internal Dashboard');
		expect(entry?.metadata).toMatchObject({
			can_be_added: true,
			number: 17443,
			protocol: 'Tcp',
			custom_known_port_id: custom.id
		});
		expect(ports.getItem('Ssh')?.metadata.custom_known_port_id).toBeUndefined();
	});

	it('replaces the custom layer so renames and deletions do not linger', () => {
		applyCustomKnownPorts([custom]);
		applyCustomKnownPorts([{ ...custom, name: 'Renamed Dashboard' }]);
		expect(get(metadata).ports).toHaveLength(builtInCount + 1);
		expect(ports.getItem(custom.id)?.name).toBe('Renamed Dashboard');

		applyCustomKnownPorts([]);
		expect(ports.getItem(custom.id)).toBeNull();
	});

	it('resolves a host port to its custom definition by endpoint, not by type', () => {
		applyCustomKnownPorts([custom]);
		const entries = ports.getItems() as PortTypeEntry[];

		expect(findCustomKnownPort(entries, 17443, 'Tcp')?.name).toBe('Internal Dashboard');
		expect(findCustomKnownPort(entries, 17443, 'Udp')).toBeNull();
		expect(findCustomKnownPort(entries, 22, 'Tcp')).toBeNull();
	});
});

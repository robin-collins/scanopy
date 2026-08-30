import type { components } from '$lib/api/schema';

export type KnownPort = components['schemas']['KnownPort'];
export type KnownPortInput = components['schemas']['KnownPortInput'];

export function createDefaultKnownPort(): KnownPortInput {
	return {
		name: '',
		description: null,
		port_number: 1,
		transport_protocol: 'Tcp'
	};
}

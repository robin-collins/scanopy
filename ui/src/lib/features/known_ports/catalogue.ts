/**
 * Known Ports catalogue projection (backend-owned merge).
 *
 * `GET /api/v1/known-ports` returns the built-in `PortType` catalogue merged
 * with the caller's organization's custom definitions. Built-ins already live
 * in the generated `ports` metadata fixture, so only the custom layer is
 * projected here. Each custom entry becomes a selectable port type in the host
 * ports picker and a name source for port displays, keyed by the custom row id
 * and tagged with `custom_known_port_id` so consumers can tell it apart from a
 * compile-time `PortType`.
 *
 * A host port created from a custom entry is still `type: 'Custom'` on the
 * wire (the backend enum has no variant for it), so displays resolve custom
 * names by `(number, protocol)` rather than by `type`.
 */

import { apiClient } from '$lib/api/client';
import {
	metadata,
	type PortTypeMetadata,
	type TypedTypeMetadata
} from '$lib/shared/stores/metadata';
import type { KnownPort } from './types';

export type PortTypeEntry = TypedTypeMetadata<PortTypeMetadata>;

export async function fetchKnownPorts(): Promise<KnownPort[]> {
	const { data } = await apiClient.GET('/api/v1/known-ports', {});
	if (!data?.success || !data.data) {
		throw new Error(data?.error || 'Failed to fetch known ports');
	}
	return data.data;
}

function toPortTypeEntry(port: KnownPort): PortTypeEntry {
	return {
		id: port.id,
		name: port.name,
		description: port.description ?? null,
		category: null,
		icon: 'binary',
		color: 'Sky',
		metadata: {
			can_be_added: true,
			is_custom: false,
			is_dns: false,
			is_management: false,
			number: port.port_number,
			protocol: port.transport_protocol,
			custom_known_port_id: port.id
		}
	};
}

/**
 * Replace the custom layer of the `ports` registry with `knownPorts`' custom
 * entries. Built-in entries are left untouched; previously projected custom
 * entries are dropped first so renames and deletions do not linger.
 */
export function applyCustomKnownPorts(knownPorts: KnownPort[]): void {
	const customEntries = knownPorts.filter((port) => port.source === 'Custom').map(toPortTypeEntry);

	metadata.update(($metadata) => {
		const builtIn = (($metadata.ports ?? []) as PortTypeEntry[]).filter(
			(item) => !item.metadata?.custom_known_port_id
		);
		return { ...$metadata, ports: [...builtIn, ...customEntries] };
	});
}

/** Fetch the merged catalogue and apply its custom layer to the registry. */
export async function loadKnownPortsIntoMetadata(): Promise<void> {
	applyCustomKnownPorts(await fetchKnownPorts());
}

/** The custom known port, if any, that describes a host port's endpoint. */
export function findCustomKnownPort(
	entries: readonly PortTypeEntry[],
	number: number,
	protocol: string
): PortTypeEntry | null {
	return (
		entries.find(
			(entry) =>
				entry.metadata?.custom_known_port_id &&
				entry.metadata.number === number &&
				entry.metadata.protocol === protocol
		) ?? null
	);
}

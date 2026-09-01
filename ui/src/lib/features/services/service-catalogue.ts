/**
 * Merged service catalogue (backend-owned merge).
 *
 * The backend's `GET /api/v1/service-catalogue` is the single source of truth
 * for the service catalogue: built-in definitions (compile-time, read-only)
 * followed by user-created custom definitions (DB rows, full CRUD). The backend
 * enforces the built-in/custom collision rule and the namespace discriminator,
 * so this store only *consumes* the merged list.
 *
 * `applyCustomServiceDefinitions` is the one UI merge point: custom entries are
 * projected onto the metadata store's `service_definitions` registry, so every
 * existing consumer of `serviceDefinitions` (service pickers, manual Unclaimed
 * port classification, per-host overrides, icon rendering) sees custom services
 * without each consumer duplicating the merge rule.
 */

import { apiClient } from '$lib/api/client';
import type { components } from '$lib/api/schema';
import { metadata } from '$lib/shared/stores/metadata';
import type { TypeMetadata } from '$lib/shared/stores/metadata';

export type ServiceCatalogueEntry = components['schemas']['ServiceCatalogueEntry'];
export type ServiceCatalogueEntryKind = components['schemas']['ServiceCatalogueEntryKind'];

/**
 * Fetch the merged catalogue from the backend. The backend owns the merge
 * order and the built-in/custom distinction.
 */
export async function fetchServiceCatalogue(): Promise<ServiceCatalogueEntry[]> {
	const { data, error } = await apiClient.GET('/api/v1/service-catalogue');
	if (error) {
		throw new Error('Failed to fetch service catalogue');
	}
	return data ?? [];
}

/**
 * Project custom catalogue entries onto the metadata registry's
 * `service_definitions` so every existing `serviceDefinitions` consumer sees
 * them. Built-in entries are left untouched (they come from the generated
 * fixture); custom entries are appended with the same `TypeMetadata` shape.
 */
export function applyCustomServiceDefinitions(catalogue: ServiceCatalogueEntry[]): void {
	const customEntries = catalogue
		.filter((entry) => entry.kind === 'custom' && entry.custom_id != null)
		.map((entry): TypeMetadata => {
			const logoUrl = entry.logo_url.trim();
			const logoExt = logoUrl ? (logoUrl.split('.').pop()?.split('?')[0] ?? 'svg') : 'svg';
			return {
				id: entry.id,
				name: entry.name,
				description: entry.description,
				category: entry.category,
				color: (entry.color ?? null) as TypeMetadata['color'],
				icon: entry.icon ?? null,
				metadata: {
					can_be_added: true,
					is_gateway: false,
					is_generic: entry.is_generic,
					has_logo: logoUrl !== '',
					logo_ext: logoExt,
					// Built-in logos are files named after the service slug; a custom
					// entry's logo is whatever URL or static path the user entered, so
					// the icon resolver uses this verbatim instead of deriving a path.
					logo_url: logoUrl,
					logo_needs_white_background: entry.logo_needs_white_background,
					has_raw_socket_endpoint: false,
					custom_service_definition_id: entry.custom_id
				}
			};
		});

	metadata.update(($metadata) => {
		const builtIn = $metadata.service_definitions ?? [];
		const builtInIds = new Set(builtIn.map((item) => item.id));
		const toAdd = customEntries.filter((entry) => !builtInIds.has(entry.id));
		return {
			...$metadata,
			service_definitions: [...builtIn, ...toAdd]
		};
	});
}

/**
 * Fetch the merged catalogue and apply it to the metadata registry. Idempotent
 * when called repeatedly (custom entries already present are skipped by id).
 */
export async function loadServiceCatalogueIntoMetadata(): Promise<void> {
	const catalogue = await fetchServiceCatalogue();
	applyCustomServiceDefinitions(catalogue);
}

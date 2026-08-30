import type { components } from '$lib/api/schema';
import type { ServiceCatalogueEntry } from '$lib/features/services/service-catalogue';

export type ServiceRefKind = components['schemas']['ServiceRefKind'];

export interface ServiceReference {
	kind: ServiceRefKind;
	id: string;
}

export interface ServiceReferenceOption {
	value: string;
	label: string;
}

export function referenceForCatalogueEntry(entry: ServiceCatalogueEntry): ServiceReference | null {
	if (entry.kind === 'built_in') {
		return { kind: 'BuiltIn', id: entry.id };
	}
	if (entry.custom_id) {
		return { kind: 'Custom', id: entry.custom_id };
	}
	return null;
}

export function encodeServiceReference(reference: ServiceReference | null): string {
	return reference ? JSON.stringify([reference.kind, reference.id]) : '';
}

export function decodeServiceReference(value: string): ServiceReference | null {
	if (!value) return null;
	try {
		const parsed: unknown = JSON.parse(value);
		if (
			!Array.isArray(parsed) ||
			parsed.length !== 2 ||
			(parsed[0] !== 'BuiltIn' && parsed[0] !== 'Custom') ||
			typeof parsed[1] !== 'string' ||
			parsed[1].length === 0
		) {
			return null;
		}
		return { kind: parsed[0], id: parsed[1] };
	} catch {
		return null;
	}
}

/**
 * Catalogue options plus a synthetic option for a stored reference that no
 * longer resolves. The raw id stays visible and selectable, so corrupt or
 * externally-written data cannot break the host editor or disappear silently.
 */
export function buildServiceReferenceOptions(
	catalogue: ServiceCatalogueEntry[],
	selected: ServiceReference | null,
	unknownLabel: string
): ServiceReferenceOption[] {
	const options = catalogue.flatMap((entry) => {
		const reference = referenceForCatalogueEntry(entry);
		return reference ? [{ value: encodeServiceReference(reference), label: entry.name }] : [];
	});
	const selectedValue = encodeServiceReference(selected);
	if (selected && !options.some((option) => option.value === selectedValue)) {
		options.push({ value: selectedValue, label: `${unknownLabel}: ${selected.id}` });
	}
	return options;
}

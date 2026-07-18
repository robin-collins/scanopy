import type { FieldDefinition } from '$lib/shared/stores/metadata';

/**
 * Build the values that metadata-driven credential fields register with TanStack Form.
 *
 * CredentialForm keeps its display values in local Svelte state, so every dynamic field
 * must also be seeded into the form. Otherwise untouched defaults (for example SSH port
 * 22) and untouched edit values validate as undefined even though they are visible.
 */
export function getCredentialFormFieldValues(
	fields: FieldDefinition[],
	fieldValues: Record<string, string>
): Record<string, string> {
	return Object.fromEntries(
		fields.map((field) => [field.id, fieldValues[field.id] ?? field.default_value ?? ''])
	);
}

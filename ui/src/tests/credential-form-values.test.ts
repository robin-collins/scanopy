import { describe, expect, it } from 'vitest';
import { getCredentialFormFieldValues } from '$lib/features/credentials/credential-form-values';
import type { FieldDefinition } from '$lib/shared/stores/metadata';

const sshFields: FieldDefinition[] = [
	{
		id: 'username',
		label: 'Username',
		field_type: 'string',
		secret: false,
		optional: false
	},
	{
		id: 'private_key',
		label: 'Private Key',
		field_type: 'secretpathorinline',
		secret: true,
		optional: false
	},
	{
		id: 'port',
		label: 'SSH Port',
		field_type: 'string',
		secret: false,
		optional: false,
		default_value: '22'
	}
];

describe('credential dynamic form values', () => {
	it('seeds defaults for untouched non-select fields', () => {
		expect(getCredentialFormFieldValues(sshFields, {})).toEqual({
			username: '',
			private_key: '',
			port: '22'
		});
	});

	it('seeds untouched edit values instead of replacing them with defaults', () => {
		expect(
			getCredentialFormFieldValues(sshFields, {
				username: 'collector',
				private_key: '********',
				port: '2222'
			})
		).toEqual({
			username: 'collector',
			private_key: '********',
			port: '2222'
		});
	});
});

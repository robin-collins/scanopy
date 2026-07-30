import { describe, expect, it } from 'vitest';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

const testDirectory = path.dirname(fileURLToPath(import.meta.url));

function readJson<T>(relativePath: string): T {
	return JSON.parse(fs.readFileSync(path.resolve(testDirectory, relativePath), 'utf8')) as T;
}

describe('UniFi credential metadata and schema', () => {
	const credentialTypes = readJson<
		Array<{
			id: string;
			metadata: {
				fields: Array<{
					id: string;
					field_type: string;
					secret: boolean;
					optional: boolean;
					default_value?: string;
					options?: Array<{ value: string }>;
				}>;
			};
		}>
	>('../lib/data/credential-types.json');
	const unifi = credentialTypes.find((credential) => credential.id === 'UnifiPassword');

	it('exposes only the supported local-controller password fields with verified TLS by default', () => {
		expect(unifi).toBeDefined();
		const fields = unifi!.metadata.fields;
		expect(fields.map((field) => field.id)).toEqual([
			'controller_url',
			'server_name',
			'site',
			'api_type',
			'tls_policy',
			'username',
			'password'
		]);
		expect(fields.find((field) => field.id === 'site')?.default_value).toBe('default');
		expect(fields.find((field) => field.id === 'api_type')?.default_value).toBe('Modern');
		expect(fields.find((field) => field.id === 'tls_policy')).toMatchObject({
			default_value: 'Verify',
			options: [{ value: 'Verify' }, { value: 'AllowInvalidCertificate' }]
		});
		expect(fields.find((field) => field.id === 'password')).toMatchObject({
			field_type: 'secretpathorinline',
			secret: true,
			optional: false
		});
		expect(fields.some((field) => /token|cookie|csrf|authorization/i.test(field.id))).toBe(false);
	});

	it('keeps the generated API credential variant password-only', () => {
		const openApi = readJson<{
			components: {
				schemas: {
					CredentialType: {
						oneOf: Array<{
							properties: Record<string, unknown> & {
								type?: { enum?: string[] };
							};
						}>;
					};
					UnifiTlsPolicy: { enum: string[] };
				};
			};
		}>('../../static/openapi.json');
		const variants = openApi.components.schemas.CredentialType.oneOf as Array<{
			properties: Record<string, unknown> & { type?: { enum?: string[] } };
		}>;
		const variant = variants.find((candidate) =>
			candidate.properties.type?.enum?.includes('UnifiPassword')
		);

		expect(variant).toBeDefined();
		expect(Object.keys(variant!.properties).sort()).toEqual(
			[
				'api_type',
				'controller_url',
				'password',
				'server_name',
				'site',
				'tls_policy',
				'type',
				'username'
			].sort()
		);
		expect(openApi.components.schemas.UnifiTlsPolicy.enum).toEqual([
			'Verify',
			'AllowInvalidCertificate'
		]);
	});
});

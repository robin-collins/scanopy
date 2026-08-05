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
	const unifiApiKey = credentialTypes.find((credential) => credential.id === 'UnifiApiKey');
	const unifiLocalAdmin = credentialTypes.find((credential) => credential.id === 'UnifiLocalAdmin');

	it('exposes only the supported API-key fields, keyed to the controller port and site', () => {
		expect(unifiApiKey).toBeDefined();
		const fields = unifiApiKey!.metadata.fields;
		expect(fields.map((field) => field.id)).toEqual(['port', 'site', 'api_key']);
		expect(fields.find((field) => field.id === 'port')?.default_value).toBe('443');
		expect(fields.find((field) => field.id === 'site')?.default_value).toBe('default');
		expect(fields.find((field) => field.id === 'api_key')).toMatchObject({
			field_type: 'secretpathorinline',
			secret: true,
			optional: false
		});
		expect(fields.some((field) => /token|cookie|csrf|authorization/i.test(field.id))).toBe(false);
	});

	it('exposes only the supported local-admin fields, keyed to the controller port and site', () => {
		expect(unifiLocalAdmin).toBeDefined();
		const fields = unifiLocalAdmin!.metadata.fields;
		expect(fields.map((field) => field.id)).toEqual(['port', 'site', 'username', 'password']);
		expect(fields.find((field) => field.id === 'port')?.default_value).toBe('443');
		expect(fields.find((field) => field.id === 'site')?.default_value).toBe('default');
		expect(fields.find((field) => field.id === 'password')).toMatchObject({
			field_type: 'secretpathorinline',
			secret: true,
			optional: false
		});
		expect(fields.some((field) => /token|cookie|csrf|authorization/i.test(field.id))).toBe(false);
	});

	it('keeps the generated API credential variants scoped to their own auth material', () => {
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
				};
			};
		}>('../../static/openapi.json');
		const variants = openApi.components.schemas.CredentialType.oneOf;

		const apiKeyVariant = variants.find((candidate) =>
			candidate.properties.type?.enum?.includes('UnifiApiKey')
		);
		expect(apiKeyVariant).toBeDefined();
		expect(Object.keys(apiKeyVariant!.properties).sort()).toEqual(
			['api_key', 'port', 'site', 'type'].sort()
		);

		const localAdminVariant = variants.find((candidate) =>
			candidate.properties.type?.enum?.includes('UnifiLocalAdmin')
		);
		expect(localAdminVariant).toBeDefined();
		expect(Object.keys(localAdminVariant!.properties).sort()).toEqual(
			['password', 'port', 'site', 'type', 'username'].sort()
		);
	});
});

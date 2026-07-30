import { describe, expect, it } from 'vitest';
import { pemPrivateKey } from '$lib/shared/components/forms/validators';

describe('pemPrivateKey', () => {
	it.each([
		['OPENSSH', '-----BEGIN OPENSSH PRIVATE KEY-----', '-----END OPENSSH PRIVATE KEY-----'],
		['PKCS#8', '-----BEGIN PRIVATE KEY-----', '-----END PRIVATE KEY-----'],
		['RSA', '-----BEGIN RSA PRIVATE KEY-----', '-----END RSA PRIVATE KEY-----'],
		['EC', '-----BEGIN EC PRIVATE KEY-----', '-----END EC PRIVATE KEY-----']
	])('accepts a matching %s envelope', (_name, begin, end) => {
		expect(pemPrivateKey(`${begin}\nkey-data\n${end}`)).toBeUndefined();
	});

	it('rejects mismatched private-key envelope tags', () => {
		expect(
			pemPrivateKey('-----BEGIN RSA PRIVATE KEY-----\nkey-data\n-----END EC PRIVATE KEY-----')
		).toBeDefined();
	});
});

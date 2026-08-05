import { describe, expect, it } from 'vitest';
import { missingDaemonFeature } from '$lib/features/credentials/utils/featureGate';

describe('credential daemon feature gate', () => {
	it('fails closed for a connected daemon missing a required build feature', () => {
		expect(missingDaemonFeature(['active_directory_gssapi'], [])).toBe('active_directory_gssapi');
	});

	it('accepts an explicitly advertised feature and preserves create-daemon flow', () => {
		expect(
			missingDaemonFeature(['active_directory_gssapi'], ['active_directory_gssapi'])
		).toBeUndefined();
		expect(missingDaemonFeature(['active_directory_gssapi'], null)).toBeUndefined();
	});
});

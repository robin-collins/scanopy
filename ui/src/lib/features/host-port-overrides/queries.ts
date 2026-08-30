/**
 * TanStack Query hooks for per-host port overrides (issue #10).
 *
 * Overrides are keyed by the value tuple (host_id, port_number, port_protocol)
 * so they survive rescans that recreate port rows. The backend resolves
 * network_id and validates every field.
 */

import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
import { apiClient } from '$lib/api/client';
import { queryKeys } from '$lib/api/query-client';
import type { components } from '$lib/api/schema';

export type HostPortOverride = components['schemas']['HostPortOverride'];
export type HostPortOverrideInput = components['schemas']['HostPortOverrideInput'];
export type ServiceRefKind = components['schemas']['ServiceRefKind'];

/**
 * Query hook for the list of overrides for a single host.
 *
 * `hostId` is a getter so the query key stays reactive to the host being edited
 * (Svelte 5 `state_referenced_locally`).
 */
export function useHostPortOverridesQuery(hostId: () => string) {
	return createQuery(() => ({
		queryKey: queryKeys.hostPortOverrides.byHost(hostId()),
		queryFn: async () => {
			const { data } = await apiClient.GET('/api/v1/host-port-overrides', {
				params: { query: { host_id: hostId() } }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to load host port overrides');
			}
			return data.data as HostPortOverride[];
		},
		enabled: !!hostId()
	}));
}

/**
 * Mutation hook for upserting a single per-host port override.
 */
export function useUpsertHostPortOverrideMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (input: HostPortOverrideInput) => {
			const { data } = await apiClient.PUT('/api/v1/host-port-overrides', { body: input });
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to save port override');
			}
			return data.data as HostPortOverride;
		},
		onSuccess: (override: HostPortOverride) => {
			queryClient.invalidateQueries({
				queryKey: queryKeys.hostPortOverrides.byHost(override.host_id)
			});
		}
	}));
}

/**
 * Mutation hook for clearing (removing) a per-host port override, restoring the
 * global default.
 */
export function useClearHostPortOverrideMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (args: { host_id: string; port_number: number; port_protocol: string }) => {
			const { data } = await apiClient.DELETE(
				'/api/v1/host-port-overrides/{host_id}/{port_number}/{port_protocol}',
				{
					params: {
						path: {
							host_id: args.host_id,
							port_number: args.port_number,
							port_protocol: args.port_protocol
						}
					}
				}
			);
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to clear port override');
			}
			return data;
		},
		onSuccess: (_result, args) => {
			queryClient.invalidateQueries({
				queryKey: queryKeys.hostPortOverrides.byHost(args.host_id)
			});
		}
	}));
}

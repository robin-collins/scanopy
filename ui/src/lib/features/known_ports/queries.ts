import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
import { apiClient } from '$lib/api/client';
import { queryKeys } from '$lib/api/query-client';
import { applyCustomKnownPorts, fetchKnownPorts } from './catalogue';
import type { KnownPort, KnownPortInput } from './types';

export function useKnownPortsQuery() {
	return createQuery(() => ({
		queryKey: queryKeys.knownPorts.all,
		queryFn: fetchKnownPorts
	}));
}

/**
 * Keep the `ports` metadata registry (host ports picker, port displays) in
 * step with the cached catalogue after every custom-port mutation.
 */
function syncCatalogue(queryClient: ReturnType<typeof useQueryClient>, next: KnownPort[]) {
	queryClient.setQueryData<KnownPort[]>(queryKeys.knownPorts.all, next);
	applyCustomKnownPorts(next);
}

export function useCreateKnownPortMutation() {
	const queryClient = useQueryClient();
	return createMutation(() => ({
		mutationFn: async (input: KnownPortInput) => {
			const { data } = await apiClient.POST('/api/v1/known-ports', { body: input });
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to create known port');
			}
			return data.data;
		},
		onSuccess: (created: KnownPort) => {
			const current = queryClient.getQueryData<KnownPort[]>(queryKeys.knownPorts.all) ?? [];
			syncCatalogue(queryClient, [...current, created]);
		}
	}));
}

export function useUpdateKnownPortMutation() {
	const queryClient = useQueryClient();
	return createMutation(() => ({
		mutationFn: async ({ id, input }: { id: string; input: KnownPortInput }) => {
			const { data } = await apiClient.PUT('/api/v1/known-ports/custom/{id}', {
				params: { path: { id } },
				body: input
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to update known port');
			}
			return data.data;
		},
		onSuccess: (updated: KnownPort) => {
			const current = queryClient.getQueryData<KnownPort[]>(queryKeys.knownPorts.all) ?? [];
			syncCatalogue(
				queryClient,
				current.map((port) => (port.id === updated.id ? updated : port))
			);
		}
	}));
}

export function useDeleteKnownPortMutation() {
	const queryClient = useQueryClient();
	return createMutation(() => ({
		mutationFn: async (id: string) => {
			const { data } = await apiClient.DELETE('/api/v1/known-ports/custom/{id}', {
				params: { path: { id } }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete known port');
			}
			return id;
		},
		onSuccess: (id: string) => {
			const current = queryClient.getQueryData<KnownPort[]>(queryKeys.knownPorts.all) ?? [];
			syncCatalogue(
				queryClient,
				current.filter((port) => port.id !== id)
			);
		}
	}));
}

/**
 * TanStack Query hooks for Services
 *
 * Services are populated by the hosts query but also have direct CRUD operations.
 */

import {
	createQuery,
	createMutation,
	useQueryClient,
	keepPreviousData
} from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import type { Service } from './types/base';
import { utcTimeZoneSentinel } from '$lib/shared/utils/formatting';
import { v4 as uuidv4 } from 'uuid';
import type { components } from '$lib/api/schema';
import { fetchServiceCatalogue, loadServiceCatalogueIntoMetadata } from './service-catalogue';

// Re-export type for convenience
export type { Service };

/**
 * Query parameters for services query including pagination and ordering
 */
export interface ServicesQueryParams {
	limit?: number;
	offset?: number;
	network_id?: string;
	host_id?: string;
	/** Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
	group_by?: components['schemas']['ServiceOrderField'];
	/** Secondary ordering field (sorting within groups or standalone sort). */
	order_by?: components['schemas']['ServiceOrderField'];
	/** Direction for order_by field. */
	order_direction?: components['schemas']['OrderDirection'];
	/** Filter by tag IDs (returns services that have ANY of the specified tags). */
	tag_ids?: string[];
	/** `true` returns only services discovery hasn't observed within their
	 * network's staleness window; omit for no staleness constraint. */
	stale?: boolean;
	/** Free-text search across service name, service definition, and the name
	 * of the host the service runs on. */
	search?: string;
	/** Only services exposed on one of these port numbers, over either protocol. */
	ports?: number[];
	/** Exclude services belonging to these categories. */
	exclude_categories?: components['schemas']['ServiceCategory'][];
}

/**
 * Pagination metadata from API response. Derived from the generated schema so
 * fields the server adds (e.g. per-group totals) reach consumers automatically.
 */
export type PaginationMeta = components['schemas']['PaginationMeta'];

/**
 * Result of a paginated query
 */
export interface PaginatedResult<T> {
	items: T[];
	pagination: PaginationMeta | null;
}

/**
 * Query hook for accessing the services cache with pagination and ordering
 * This cache is primarily populated by useHostsQuery
 *
 * @param paramsOrGetter - Query parameters or getter function returning parameters.
 *                         Use getter function for reactive options (e.g., when offset or ordering changes).
 */
export function useServicesQuery(
	paramsOrGetter: ServicesQueryParams | (() => ServicesQueryParams) = {}
) {
	return createQuery(() => {
		const params = typeof paramsOrGetter === 'function' ? paramsOrGetter() : paramsOrGetter;
		const {
			limit,
			offset,
			network_id,
			host_id,
			group_by,
			order_by,
			order_direction,
			tag_ids,
			stale,
			search,
			ports,
			exclude_categories
		} = params;

		return {
			queryKey: [
				...queryKeys.services.all,
				{
					limit,
					offset,
					network_id,
					host_id,
					group_by,
					order_by,
					order_direction,
					tag_ids,
					stale,
					search,
					ports,
					exclude_categories
				}
			],
			queryFn: async (): Promise<PaginatedResult<Service>> => {
				const { data } = await apiClient.GET('/api/v1/services', {
					params: {
						query: {
							limit,
							offset,
							network_id,
							host_id,
							group_by,
							order_by,
							order_direction,
							tag_ids,
							stale,
							search,
							ports,
							exclude_categories
						}
					}
				});
				if (!data?.success || !data.data) {
					throw new Error(data?.error || 'Failed to fetch services');
				}
				return {
					items: data.data,
					pagination: data.meta?.pagination ?? null
				};
			},
			// Keep showing previous page data while fetching next page
			placeholderData: keepPreviousData
		};
	});
}

/**
 * Query hook for accessing the services cache populated by useHostsQuery.
 * This does NOT make API calls - it reads from the cache that hosts query populates.
 * Use this when you need all services for filtering (e.g., by host_id in HostCard).
 *
 * For paginated/filtered API calls, use useServicesQuery() instead.
 */
export function useServicesCacheQuery() {
	return createQuery(() => ({
		queryKey: queryKeys.services.all,
		initialData: [] as Service[],
		staleTime: Infinity,
		refetchOnMount: false,
		refetchOnWindowFocus: false,
		enabled: false
	}));
}

/**
 * Query hook for fetching specific services by IDs (for selective loading)
 * Used for lookups where only a subset of services is needed (e.g., virtualization lookups)
 *
 * @param idsGetter - Getter function returning array of service IDs to fetch
 */
export function useServicesByIds(idsGetter: () => string[]) {
	return createQuery(() => {
		const ids = idsGetter();

		return {
			queryKey: [...queryKeys.services.all, 'byIds', ids],
			queryFn: async (): Promise<Service[]> => {
				if (ids.length === 0) return [];

				const { data } = await apiClient.GET('/api/v1/services', {
					params: {
						query: {
							ids: ids,
							limit: 0 // No pagination when fetching by IDs
						}
					}
				});
				if (!data?.success || !data.data) {
					throw new Error(data?.error || 'Failed to fetch services');
				}

				return data.data;
			},
			enabled: ids.length > 0
		};
	});
}

/**
 * Mutation hook for creating a service
 */
export function useCreateServiceMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (service: Service) => {
			const { data } = await apiClient.POST('/api/v1/services', { body: service });
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to create service');
			}
			return data.data;
		},
		onSuccess: (newService: Service) => {
			queryClient.setQueryData<Service[]>(queryKeys.services.all, (old) =>
				old ? [...old, newService] : [newService]
			);
		}
	}));
}

/**
 * Mutation hook for updating a service
 */
export function useUpdateServiceMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (service: Service) => {
			const { data } = await apiClient.PUT('/api/v1/services/{id}', {
				params: { path: { id: service.id } },
				body: service
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to update service');
			}
			return data.data;
		},
		onSuccess: (updatedService: Service) => {
			queryClient.setQueryData<Service[]>(
				queryKeys.services.all,
				(old) => old?.map((s) => (s.id === updatedService.id ? updatedService : s)) ?? []
			);
			// Invalidate paginated service queries so ServiceTab reflects the update
			queryClient.invalidateQueries({ queryKey: queryKeys.services.all });
		}
	}));
}

/**
 * Mutation hook for deleting a service
 */
export function useDeleteServiceMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (id: string) => {
			const { data } = await apiClient.DELETE('/api/v1/services/{id}', {
				params: { path: { id } }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete service');
			}
			return id;
		},
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: queryKeys.services.all });
		}
	}));
}

/**
 * Mutation hook for bulk deleting services
 */
export function useBulkDeleteServicesMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (ids: string[]) => {
			const { data } = await apiClient.POST('/api/v1/services/bulk-delete', { body: ids });
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete services');
			}
			return ids;
		},
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: queryKeys.services.all });
		}
	}));
}

// `useBulkUpdateServicesMutation` was removed here. It had no call sites, and it
// derived its DELETE set from `getQueryData(queryKeys.services.all)` — a
// synchronous snapshot of a cache with no fetcher of its own — so any host whose
// services were not in that cache would have had its unseen services deleted.
// Host service sync already goes through `PUT /api/v1/hosts/{id}`
// (`useUpdateHostMutation`), which reconciles create/update/delete server-side
// against the real rows.

// ============================================================================
// Custom service definitions (Platform > Known Services)
// ============================================================================

export type CustomServiceDefinition = components['schemas']['CustomServiceDefinition'];

/**
 * The merged service catalogue: built-in (read-only) + custom entries, ordered
 * by the backend. The backend owns the merge.
 */
export function useServiceCatalogueQuery() {
	return createQuery(() => ({
		queryKey: queryKeys.serviceCatalogue.all,
		queryFn: fetchServiceCatalogue
	}));
}

/**
 * Query hook for custom service definitions (the CRUD layer on top of the
 * built-in catalogue). Returns the flat list of custom rows.
 */
export function useCustomServiceDefinitionsQuery() {
	return createQuery(() => ({
		queryKey: queryKeys.customServiceDefinitions.all,
		queryFn: async (): Promise<CustomServiceDefinition[]> => {
			const { data } = await apiClient.GET('/api/v1/custom-service-definitions');
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to fetch custom service definitions');
			}
			return data.data;
		}
	}));
}

/**
 * Re-fetch the merged catalogue and re-apply custom entries to the metadata
 * registry. Called after any custom-definition mutation so pickers and the
 * Known Services page reflect the change.
 */
async function refreshCatalogue(queryClient: ReturnType<typeof useQueryClient>): Promise<void> {
	await queryClient.invalidateQueries({ queryKey: queryKeys.serviceCatalogue.all });
	await queryClient.invalidateQueries({ queryKey: queryKeys.customServiceDefinitions.all });
	await loadServiceCatalogueIntoMetadata();
}

/**
 * Mutation hook for creating a custom service definition.
 */
export function useCreateCustomServiceDefinitionMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (definition: components['schemas']['CustomServiceDefinitionBase']) => {
			const now = new Date().toISOString();
			// The server regenerates the id when it receives the nil uuid.
			const body: CustomServiceDefinition = {
				...definition,
				id: '00000000-0000-0000-0000-000000000000',
				created_at: now,
				updated_at: now
			};
			const { data } = await apiClient.POST('/api/v1/custom-service-definitions', {
				body
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to create custom service definition');
			}
			return data.data;
		},
		onSuccess: async () => {
			await refreshCatalogue(queryClient);
		}
	}));
}

/**
 * Mutation hook for updating a custom service definition.
 */
export function useUpdateCustomServiceDefinitionMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (definition: CustomServiceDefinition) => {
			const { data } = await apiClient.PUT('/api/v1/custom-service-definitions/{id}', {
				params: { path: { id: definition.id } },
				body: definition
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to update custom service definition');
			}
			return data.data;
		},
		onSuccess: async () => {
			await refreshCatalogue(queryClient);
		}
	}));
}

/**
 * Mutation hook for deleting a custom service definition.
 */
export function useDeleteCustomServiceDefinitionMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (id: string) => {
			const { data } = await apiClient.DELETE('/api/v1/custom-service-definitions/{id}', {
				params: { path: { id } }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete custom service definition');
			}
			return id;
		},
		onSuccess: async () => {
			await refreshCatalogue(queryClient);
		}
	}));
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Create a default empty service for a host
 */
export function createDefaultService(
	serviceType: string,
	host_id: string,
	host_network_id: string
): Service {
	return {
		id: uuidv4(), // Generate real UUID for client-provided ID
		created_at: utcTimeZoneSentinel,
		updated_at: utcTimeZoneSentinel,
		network_id: host_network_id,
		host_id,
		tags: [],
		service_definition: serviceType,
		name: serviceType,
		bindings: [],
		virtualization_metadata: null,
		virtualization_service_id: null,
		position: 0, // Will be set by server based on existing services
		source: {
			type: 'Manual'
		}
	};
}

/**
 * Get a display name for a service
 */
export function getServiceName(service: Service): string {
	return service.name || service.service_definition;
}

/**
 * TanStack Query hooks for user-authored custom topology views: the view
 * container itself, its placed nodes/edges, the batch layout-save endpoint,
 * and the (organization-scoped) library object stencil catalog.
 *
 * Unlike the built-in L2/L3/Workloads/Application views (computed live from
 * entity data every request), a custom view's nodes and edges are hand-placed
 * and persisted as-is — see backend/src/server/custom_topology_views.
 */

import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient, getServerUrl } from '$lib/api/client';
import { pushError } from '$lib/shared/stores/feedback';
import { uuidv4Sentinel, utcTimeZoneSentinel } from '$lib/shared/utils/formatting';
import type { components } from '$lib/api/schema';

export type CustomTopologyView = components['schemas']['CustomTopologyView'];
export type CustomViewNode = components['schemas']['CustomViewNode'];
export type CustomViewEdge = components['schemas']['CustomViewEdge'];
export type LibraryObject = components['schemas']['LibraryObject'];
export type NodeKind = components['schemas']['NodeKind'];
export type NodeStyle = components['schemas']['NodeStyle'];
export type CornerStyle = components['schemas']['CornerStyle'];

/**
 * The generic create-handler macro types every entity's request body as the
 * FULL entity shape (readonly id/created_at/updated_at included), even
 * though the server's `#[serde(default)]` on those fields means it never
 * reads them from a create request — it always regenerates them. Same
 * placeholder convention as `createDefaultCredential`/`Tag` creation.
 */
function withPlaceholderMeta<T extends object>(
	base: T
): T & { id: string; created_at: string; updated_at: string } {
	return {
		...base,
		id: uuidv4Sentinel,
		created_at: utcTimeZoneSentinel,
		updated_at: utcTimeZoneSentinel
	};
}

// ============================================================================
// Custom Topology Views
// ============================================================================

export function useCustomTopologyViewsQuery(networkId: () => string | undefined) {
	return createQuery(() => ({
		queryKey: queryKeys.customTopologyViews.byNetwork(networkId() ?? ''),
		queryFn: async () => {
			const id = networkId();
			if (!id) return [] as CustomTopologyView[];
			const { data } = await apiClient.GET('/api/v1/custom-topology-views', {
				params: { query: { network_id: id, limit: 0 } }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to fetch custom topology views');
			}
			return data.data;
		},
		enabled: () => !!networkId()
	}));
}

export function useCreateCustomTopologyViewMutation() {
	const queryClient = useQueryClient();
	return createMutation(() => ({
		mutationFn: async ({ networkId, name }: { networkId: string; name: string }) => {
			const { data } = await apiClient.POST('/api/v1/custom-topology-views', {
				body: withPlaceholderMeta({ network_id: networkId, name })
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to create custom topology view');
			}
			return data.data;
		},
		onSuccess: (view) => {
			queryClient.invalidateQueries({
				queryKey: queryKeys.customTopologyViews.byNetwork(view.network_id)
			});
		}
	}));
}

export function useRenameCustomTopologyViewMutation() {
	const queryClient = useQueryClient();
	return createMutation(() => ({
		mutationFn: async ({ view, name }: { view: CustomTopologyView; name: string }) => {
			const { data } = await apiClient.PUT('/api/v1/custom-topology-views/{id}', {
				params: { path: { id: view.id } },
				body: { ...view, name }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to rename custom topology view');
			}
			return data.data;
		},
		onSuccess: (view) => {
			queryClient.invalidateQueries({
				queryKey: queryKeys.customTopologyViews.byNetwork(view.network_id)
			});
		}
	}));
}

export function useDeleteCustomTopologyViewMutation() {
	const queryClient = useQueryClient();
	return createMutation(() => ({
		mutationFn: async ({ id }: { id: string; networkId: string }) => {
			const { data } = await apiClient.DELETE('/api/v1/custom-topology-views/{id}', {
				params: { path: { id } }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete custom topology view');
			}
			return id;
		},
		onSuccess: (_id, variables) => {
			queryClient.invalidateQueries({
				queryKey: queryKeys.customTopologyViews.byNetwork(variables.networkId)
			});
		}
	}));
}

// ============================================================================
// Nodes
// ============================================================================

export function useCustomViewNodesQuery(viewId: () => string | undefined) {
	return createQuery(() => ({
		queryKey: queryKeys.customViewNodes.byView(viewId() ?? ''),
		queryFn: async () => {
			const id = viewId();
			if (!id) return [] as CustomViewNode[];
			const { data } = await apiClient.GET('/api/v1/custom-view-nodes', {
				params: { query: { view_id: id, limit: 0 } }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to fetch view nodes');
			}
			return data.data;
		},
		enabled: () => !!viewId()
	}));
}

export function useCreateCustomViewNodeMutation() {
	const queryClient = useQueryClient();
	return createMutation(() => ({
		mutationFn: async (node: Omit<CustomViewNode, 'id' | 'created_at' | 'updated_at'>) => {
			const { data } = await apiClient.POST('/api/v1/custom-view-nodes', {
				body: withPlaceholderMeta(node)
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to create node');
			}
			return data.data;
		},
		onSuccess: (node) => {
			queryClient.invalidateQueries({ queryKey: queryKeys.customViewNodes.byView(node.view_id) });
		}
	}));
}

export function useUpdateCustomViewNodeMutation() {
	const queryClient = useQueryClient();
	return createMutation(() => ({
		mutationFn: async (node: CustomViewNode) => {
			const { data } = await apiClient.PUT('/api/v1/custom-view-nodes/{id}', {
				params: { path: { id: node.id } },
				body: node
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to update node');
			}
			return data.data;
		},
		onSuccess: (node) => {
			queryClient.invalidateQueries({ queryKey: queryKeys.customViewNodes.byView(node.view_id) });
		}
	}));
}

export function useDeleteCustomViewNodeMutation() {
	const queryClient = useQueryClient();
	return createMutation(() => ({
		mutationFn: async ({ id }: { id: string; viewId: string }) => {
			const { data } = await apiClient.DELETE('/api/v1/custom-view-nodes/{id}', {
				params: { path: { id } }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete node');
			}
			return id;
		},
		onSuccess: (_id, variables) => {
			queryClient.invalidateQueries({
				queryKey: queryKeys.customViewNodes.byView(variables.viewId)
			});
			queryClient.invalidateQueries({
				queryKey: queryKeys.customViewEdges.byView(variables.viewId)
			});
		}
	}));
}

/** Path to fetch (and use as an `<img src>`) a custom-view node's uploaded image. */
export function customViewNodeImageUrl(nodeId: string): string {
	return `${getServerUrl()}/api/v1/custom-view-nodes/${nodeId}/content`;
}

/** Upload/replace a per-node custom image override. See host-images/queries.ts for why this bypasses `apiClient`. */
export function useUploadCustomViewNodeImageMutation() {
	const queryClient = useQueryClient();
	return createMutation(() => ({
		mutationFn: async ({ nodeId, file }: { nodeId: string; file: File }) => {
			const body = new FormData();
			body.append('file', file);
			const response = await fetch(`${getServerUrl()}/api/v1/custom-view-nodes/${nodeId}/image`, {
				method: 'PUT',
				credentials: 'include',
				body
			});
			const data = await response.json();
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to upload image');
			}
			return data.data as CustomViewNode;
		},
		onSuccess: (node) => {
			queryClient.invalidateQueries({ queryKey: queryKeys.customViewNodes.byView(node.view_id) });
		},
		onError: (error: Error) => pushError(error.message)
	}));
}

// ============================================================================
// Edges
// ============================================================================

export function useCustomViewEdgesQuery(viewId: () => string | undefined) {
	return createQuery(() => ({
		queryKey: queryKeys.customViewEdges.byView(viewId() ?? ''),
		queryFn: async () => {
			const id = viewId();
			if (!id) return [] as CustomViewEdge[];
			const { data } = await apiClient.GET('/api/v1/custom-view-edges', {
				params: { query: { view_id: id, limit: 0 } }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to fetch view edges');
			}
			return data.data;
		},
		enabled: () => !!viewId()
	}));
}

export function useCreateCustomViewEdgeMutation() {
	const queryClient = useQueryClient();
	return createMutation(() => ({
		mutationFn: async (edge: Omit<CustomViewEdge, 'id' | 'created_at' | 'updated_at'>) => {
			const { data } = await apiClient.POST('/api/v1/custom-view-edges', {
				body: withPlaceholderMeta(edge)
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to create edge');
			}
			return data.data;
		},
		onSuccess: (edge) => {
			queryClient.invalidateQueries({ queryKey: queryKeys.customViewEdges.byView(edge.view_id) });
		}
	}));
}

export function useDeleteCustomViewEdgeMutation() {
	const queryClient = useQueryClient();
	return createMutation(() => ({
		mutationFn: async ({ id }: { id: string; viewId: string }) => {
			const { data } = await apiClient.DELETE('/api/v1/custom-view-edges/{id}', {
				params: { path: { id } }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete edge');
			}
			return id;
		},
		onSuccess: (_id, variables) => {
			queryClient.invalidateQueries({
				queryKey: queryKeys.customViewEdges.byView(variables.viewId)
			});
		}
	}));
}

// ============================================================================
// Batch layout save (drag/resize/style auto-save)
// ============================================================================

export interface SaveLayoutParams {
	viewId: string;
	nodes: CustomViewNode[];
	edges: CustomViewEdge[];
}

/**
 * Batch-upsert node/edge positions/styles in one request — what the canvas's
 * debounced auto-save calls on drag-stop/resize-stop/edit-blur so moving
 * several nodes at once doesn't fire one HTTP round trip per node.
 */
export function useSaveCustomTopologyViewLayoutMutation() {
	const queryClient = useQueryClient();
	return createMutation(() => ({
		mutationFn: async ({ viewId, nodes, edges }: SaveLayoutParams) => {
			const { data } = await apiClient.PUT('/api/v1/custom-topology-views/{id}/layout', {
				params: { path: { id: viewId } },
				body: { nodes, edges }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to save layout');
			}
			return data.data;
		},
		onSuccess: (_result, variables) => {
			queryClient.invalidateQueries({
				queryKey: queryKeys.customViewNodes.byView(variables.viewId)
			});
			queryClient.invalidateQueries({
				queryKey: queryKeys.customViewEdges.byView(variables.viewId)
			});
		}
	}));
}

// ============================================================================
// Library Objects (built-in + org-owned stencil catalog)
// ============================================================================

export function useLibraryObjectsQuery() {
	return createQuery(() => ({
		queryKey: queryKeys.libraryObjects.all,
		queryFn: async () => {
			const { data } = await apiClient.GET('/api/v1/library-objects', {});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to fetch library objects');
			}
			return data.data;
		}
	}));
}

export function useCreateLibraryObjectMutation() {
	const queryClient = useQueryClient();
	return createMutation(() => ({
		mutationFn: async (object: Pick<LibraryObject, 'name' | 'icon' | 'color'>) => {
			const { data } = await apiClient.POST('/api/v1/library-objects', {
				body: withPlaceholderMeta(object)
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to create library object');
			}
			return data.data;
		},
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: queryKeys.libraryObjects.all });
		}
	}));
}

export function useUpdateLibraryObjectMutation() {
	const queryClient = useQueryClient();
	return createMutation(() => ({
		mutationFn: async (object: LibraryObject) => {
			const { data } = await apiClient.PUT('/api/v1/library-objects/{id}', {
				params: { path: { id: object.id } },
				body: object
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to update library object');
			}
			return data.data;
		},
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: queryKeys.libraryObjects.all });
		}
	}));
}

export function useDeleteLibraryObjectMutation() {
	const queryClient = useQueryClient();
	return createMutation(() => ({
		mutationFn: async (id: string) => {
			const { data } = await apiClient.DELETE('/api/v1/library-objects/{id}', {
				params: { path: { id } }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete library object');
			}
			return id;
		},
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: queryKeys.libraryObjects.all });
		}
	}));
}

export function libraryObjectImageUrl(objectId: string): string {
	return `${getServerUrl()}/api/v1/library-objects/${objectId}/content`;
}

export function useUploadLibraryObjectImageMutation() {
	const queryClient = useQueryClient();
	return createMutation(() => ({
		mutationFn: async ({ objectId, file }: { objectId: string; file: File }) => {
			const body = new FormData();
			body.append('file', file);
			const response = await fetch(`${getServerUrl()}/api/v1/library-objects/${objectId}/image`, {
				method: 'PUT',
				credentials: 'include',
				body
			});
			const data = await response.json();
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to upload image');
			}
			return data.data as LibraryObject;
		},
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: queryKeys.libraryObjects.all });
		},
		onError: (error: Error) => pushError(error.message)
	}));
}

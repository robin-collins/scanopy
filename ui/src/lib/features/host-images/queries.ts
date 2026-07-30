/**
 * TanStack Query hooks for a host's image gallery.
 *
 * Unlike ports/interfaces, HostImage isn't embedded in HostResponse (gallery
 * data isn't needed every time a host loads), so this fetches directly
 * rather than reading out of a cache populated by the hosts query.
 */

import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient, getServerUrl } from '$lib/api/client';
import { pushError } from '$lib/shared/stores/feedback';
import type { components } from '$lib/api/schema';

export type HostImage = components['schemas']['HostImage'];

/** Path to fetch (and use as an `<img src>`) an image's raw bytes. */
export function hostImageContentUrl(imageId: string): string {
	return `${getServerUrl()}/api/v1/host-images/${imageId}/content`;
}

/** Query hook: list a host's gallery images. */
export function useHostImagesQuery(hostId: () => string | undefined) {
	return createQuery(() => ({
		queryKey: queryKeys.hostImages.byHost(hostId() ?? ''),
		queryFn: async () => {
			const id = hostId();
			if (!id) return [] as HostImage[];
			const { data } = await apiClient.GET('/api/v1/host-images', {
				params: { query: { host_id: id, limit: 0 } }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to fetch host images');
			}
			return data.data;
		},
		enabled: () => !!hostId()
	}));
}

/**
 * Mutation hook: upload a new gallery image. Bypasses the shared
 * `apiClient` (which defaults every request to `Content-Type:
 * application/json`) and uses `fetch` directly so the browser can set the
 * multipart boundary itself — an explicit JSON content-type on a
 * multipart body would make axum's `Multipart` extractor reject the request.
 */
export function useUploadHostImageMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async ({ hostId, file }: { hostId: string; file: File }) => {
			const body = new FormData();
			body.append('host_id', hostId);
			body.append('file', file);

			const response = await fetch(`${getServerUrl()}/api/v1/host-images`, {
				method: 'POST',
				credentials: 'include',
				body
			});
			const data = await response.json();
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to upload image');
			}
			return data.data as HostImage;
		},
		onSuccess: (image) => {
			queryClient.invalidateQueries({ queryKey: queryKeys.hostImages.byHost(image.host_id) });
		},
		// This mutation bypasses `apiClient` (see above), so it also bypasses
		// its error middleware's automatic toast — surface failures manually.
		onError: (error: Error) => pushError(error.message)
	}));
}

/** Mutation hook: delete a gallery image. */
export function useDeleteHostImageMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async ({ imageId }: { imageId: string; hostId: string }) => {
			const { data } = await apiClient.DELETE('/api/v1/host-images/{id}', {
				params: { path: { id: imageId } }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete image');
			}
			return imageId;
		},
		onSuccess: (_id, variables) => {
			queryClient.invalidateQueries({ queryKey: queryKeys.hostImages.byHost(variables.hostId) });
		}
	}));
}

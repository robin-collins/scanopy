/**
 * TanStack Query hooks for Categories (device categories assignable to hosts).
 */

import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import type { Category } from './types/base';

/**
 * Query hook for fetching every category visible to the caller's
 * organization: the seeded built-in catalog plus that org's own additions.
 */
export function useCategoriesQuery() {
	return createQuery(() => ({
		queryKey: queryKeys.categories.all,
		queryFn: async () => {
			const { data } = await apiClient.GET('/api/v1/categories', {});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to fetch categories');
			}
			return data.data;
		}
	}));
}

/**
 * Mutation hook for creating a category
 */
export function useCreateCategoryMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (category: Category) => {
			const { data } = await apiClient.POST('/api/v1/categories', { body: category });
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to create category');
			}
			return data.data;
		},
		onSuccess: (newCategory: Category) => {
			queryClient.setQueryData<Category[]>(queryKeys.categories.all, (old) =>
				old ? [...old, newCategory] : [newCategory]
			);
		}
	}));
}

/**
 * Mutation hook for updating a category
 */
export function useUpdateCategoryMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (category: Category) => {
			const { data } = await apiClient.PUT('/api/v1/categories/{id}', {
				params: { path: { id: category.id } },
				body: category
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to update category');
			}
			return data.data;
		},
		onSuccess: (updatedCategory: Category) => {
			queryClient.setQueryData<Category[]>(
				queryKeys.categories.all,
				(old) => old?.map((c) => (c.id === updatedCategory.id ? updatedCategory : c)) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for deleting a category. Hosts referencing it fall back to
 * uncategorized (server sets category_id to NULL via ON DELETE SET NULL).
 */
export function useDeleteCategoryMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (id: string) => {
			const { data } = await apiClient.DELETE('/api/v1/categories/{id}', {
				params: { path: { id } }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete category');
			}
			return id;
		},
		onSuccess: (id: string) => {
			queryClient.setQueryData<Category[]>(
				queryKeys.categories.all,
				(old) => old?.filter((c) => c.id !== id) ?? []
			);
			// Hosts referencing the deleted category are now uncategorized server-side.
			queryClient.invalidateQueries({ queryKey: queryKeys.hosts.lists() });
		}
	}));
}

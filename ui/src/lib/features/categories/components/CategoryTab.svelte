<script lang="ts">
	import {
		useCategoriesQuery,
		useCreateCategoryMutation,
		useUpdateCategoryMutation,
		useDeleteCategoryMutation
	} from '../queries';
	import CategoryCard from './CategoryCard.svelte';
	import CategoryEditModal from './CategoryEditModal.svelte';
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import type { Category } from '../types/base';
	import { Plus } from 'lucide-svelte';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { permissions } from '$lib/shared/stores/metadata';
	import type { TabProps } from '$lib/shared/types';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import {
		common_confirmDeleteName,
		common_create,
		common_categories,
		common_noEntityYet,
		categories_noCategoriesHelp,
		categories_subtitle
	} from '$lib/paraglide/messages';

	let { isReadOnly = false }: TabProps = $props();

	let showCategoryEditor = $state(false);
	let editingCategory: Category | null = $state(null);

	// Deep-link: open category editor from URL (handles both fresh open and entity switch)
	$effect(() => {
		const result = resolveModalDeepLink(
			$modalState,
			'category-editor',
			categories,
			showCategoryEditor,
			editingCategory?.id
		);
		if (result !== undefined) {
			editingCategory = result;
			showCategoryEditor = true;
		}
	});

	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	const categoriesQuery = useCategoriesQuery();
	const createCategoryMutation = useCreateCategoryMutation();
	const updateCategoryMutation = useUpdateCategoryMutation();
	const deleteCategoryMutation = useDeleteCategoryMutation();

	let categories = $derived(
		[...(categoriesQuery.data ?? [])].sort((a, b) => a.name.localeCompare(b.name))
	);
	let isLoading = $derived(categoriesQuery.isLoading);

	let canManage = $derived(
		!isReadOnly &&
			currentUser &&
			permissions.getMetadata(currentUser.permissions).manage_org_entities
	);

	function handleCreateCategory() {
		editingCategory = null;
		showCategoryEditor = true;
	}

	function handleEditCategory(category: Category) {
		editingCategory = category;
		showCategoryEditor = true;
	}

	async function handleDeleteCategory(category: Category) {
		if (confirm(common_confirmDeleteName({ name: category.name }))) {
			await deleteCategoryMutation.mutateAsync(category.id);
		}
	}

	async function handleCategoryCreate(data: Category) {
		await createCategoryMutation.mutateAsync(data);
		showCategoryEditor = false;
		editingCategory = null;
	}

	async function handleCategoryUpdate(_id: string, data: Category) {
		await updateCategoryMutation.mutateAsync(data);
		showCategoryEditor = false;
		editingCategory = null;
	}

	function handleCloseCategoryEditor() {
		showCategoryEditor = false;
		editingCategory = null;
	}
</script>

<div class="space-y-6">
	<TabHeader title={common_categories()} subtitle={categories_subtitle()}>
		<svelte:fragment slot="actions">
			{#if canManage}
				<button class="btn-primary flex items-center" onclick={handleCreateCategory}>
					<Plus class="h-5 w-5" />{common_create()}
				</button>
			{/if}
		</svelte:fragment>
	</TabHeader>

	{#if isLoading}
		<Loading />
	{:else if categories.length === 0}
		<EmptyState
			title={common_noEntityYet({ entity: common_categories() })}
			subtitle={categories_noCategoriesHelp()}
			onClick={canManage ? handleCreateCategory : undefined}
			cta={canManage ? common_create() : ''}
		/>
	{:else}
		<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
			{#each categories as category (category.id)}
				<CategoryCard {category} onEdit={handleEditCategory} onDelete={handleDeleteCategory} />
			{/each}
		</div>
	{/if}
</div>

<CategoryEditModal
	name="category-editor"
	isOpen={showCategoryEditor}
	category={editingCategory}
	onCreate={handleCategoryCreate}
	onUpdate={handleCategoryUpdate}
	onClose={handleCloseCategoryEditor}
	onDelete={editingCategory ? () => handleDeleteCategory(editingCategory!) : null}
/>

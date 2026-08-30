<script lang="ts">
	import { Plus, Edit, Trash2 } from 'lucide-svelte';
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import TextArea from '$lib/shared/components/forms/input/TextArea.svelte';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import Checkbox from '$lib/shared/components/forms/input/Checkbox.svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { pushError, pushSuccess } from '$lib/shared/stores/feedback';
	import type { TabProps } from '$lib/shared/types';
	import { serviceCategories, permissions } from '$lib/shared/stores/metadata';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import {
		useServiceCatalogueQuery,
		useCreateCustomServiceDefinitionMutation,
		useUpdateCustomServiceDefinitionMutation,
		useDeleteCustomServiceDefinitionMutation,
		type CustomServiceDefinition
	} from '../queries';
	import type { ServiceCatalogueEntry } from '../service-catalogue';
	import type { components } from '$lib/api/schema';
	import {
		common_builtin,
		common_cancel,
		common_category,
		common_custom,
		common_delete,
		common_description,
		common_edit,
		common_name,
		common_save,
		serviceDefinitions_addCustom,
		serviceDefinitions_createdSuccess,
		serviceDefinitions_deletedSuccess,
		serviceDefinitions_deleteConfirm,
		serviceDefinitions_editCustom,
		serviceDefinitions_emptySubtitle,
		serviceDefinitions_isGeneric,
		serviceDefinitions_isGenericHelp,
		serviceDefinitions_logoNeedsWhiteBackground,
		serviceDefinitions_logoUrl,
		serviceDefinitions_logoUrlHelp,
		serviceDefinitions_nameHelp,
		serviceDefinitions_newCustom,
		serviceDefinitions_subtitle,
		serviceDefinitions_title,
		serviceDefinitions_updatedSuccess
	} from '$lib/paraglide/messages';

	let { isReadOnly = false }: TabProps = $props();

	let isOpen = $state(false);
	let editing: CustomServiceDefinition | null = $state(null);

	// Form state
	let name = $state('');
	let description = $state('');
	let category = $state('');
	let logoUrl = $state('');
	let logoNeedsWhiteBackground = $state(false);
	let isGeneric = $state(false);

	// Minimal TanStack-form-compatible field shim so the shared input components
	// can be reused without a full form instance.
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	function fieldFor(value: any, set: (v: any) => void): any {
		return {
			state: {
				value,
				meta: { isTouched: false, errors: [] }
			},
			handleChange: (v: unknown) => set(v),
			handleBlur: () => {}
		};
	}

	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);
	let canManage = $derived(
		!isReadOnly &&
			currentUser &&
			permissions.getMetadata(currentUser.permissions).manage_org_entities
	);

	const catalogueQuery = useServiceCatalogueQuery();
	const createMutation = useCreateCustomServiceDefinitionMutation();
	const updateMutation = useUpdateCustomServiceDefinitionMutation();
	const deleteMutation = useDeleteCustomServiceDefinitionMutation();

	let entries = $derived(catalogueQuery.data ?? []);
	let isLoading = $derived(catalogueQuery.isLoading);

	let categoryOptions = $derived(
		(serviceCategories.getItems() ?? [])
			.map((item) => ({
				value: item.id,
				label: item.name ?? item.id
			}))
			.sort((a, b) => a.label.localeCompare(b.label))
	);

	function resetForm() {
		name = '';
		description = '';
		category = '';
		logoUrl = '';
		logoNeedsWhiteBackground = false;
		isGeneric = false;
	}

	function handleCreate() {
		editing = null;
		resetForm();
		isOpen = true;
	}

	function handleEdit(entry: ServiceCatalogueEntry) {
		editing = {
			id: entry.custom_id ?? '',
			created_at: '',
			updated_at: '',
			name: entry.name,
			description: entry.description,
			category: entry.category,
			logo_url: entry.logo_url,
			logo_needs_white_background: entry.logo_needs_white_background,
			is_generic: entry.is_generic
		} as CustomServiceDefinition;
		name = entry.name;
		description = entry.description;
		category = entry.category;
		logoUrl = entry.logo_url;
		logoNeedsWhiteBackground = entry.logo_needs_white_background;
		isGeneric = entry.is_generic;
		isOpen = true;
	}

	function handleClose() {
		isOpen = false;
		editing = null;
	}

	async function handleSave() {
		const trimmed = name.trim();
		if (!trimmed) {
			pushError(common_name());
			return;
		}
		if (!category) {
			pushError(common_category());
			return;
		}

		const base: components['schemas']['CustomServiceDefinitionBase'] = {
			name: trimmed,
			description: description.trim(),
			category,
			logo_url: logoUrl.trim(),
			logo_needs_white_background: logoNeedsWhiteBackground,
			is_generic: isGeneric
		};

		try {
			if (editing) {
				await updateMutation.mutateAsync({
					id: editing.id,
					created_at: editing.created_at,
					updated_at: editing.updated_at,
					...base
				});
				pushSuccess(serviceDefinitions_updatedSuccess());
			} else {
				await createMutation.mutateAsync(base);
				pushSuccess(serviceDefinitions_createdSuccess());
			}
			handleClose();
		} catch (e) {
			pushError(e instanceof Error ? e.message : String(e));
		}
	}

	async function handleDelete(entry: ServiceCatalogueEntry) {
		if (!entry.custom_id) return;
		if (confirm(serviceDefinitions_deleteConfirm({ name: entry.name }))) {
			try {
				await deleteMutation.mutateAsync(entry.custom_id);
				pushSuccess(serviceDefinitions_deletedSuccess());
			} catch (e) {
				pushError(e instanceof Error ? e.message : String(e));
			}
		}
	}
</script>

<div class="space-y-6">
	<TabHeader title={serviceDefinitions_title()} subtitle={serviceDefinitions_subtitle()}>
		<svelte:fragment slot="actions">
			{#if canManage}
				<button class="btn-primary flex items-center" onclick={handleCreate}>
					<Plus class="h-5 w-5" />{serviceDefinitions_addCustom()}
				</button>
			{/if}
		</svelte:fragment>
	</TabHeader>

	{#if isLoading}
		<Loading />
	{:else}
		<div class="overflow-x-auto rounded-lg border border-gray-700">
			<table class="w-full text-left text-sm">
				<thead class="text-muted border-b border-gray-700 bg-gray-800/60">
					<tr>
						<th class="px-4 py-3 font-medium">{common_name()}</th>
						<th class="px-4 py-3 font-medium">{common_category()}</th>
						<th class="px-4 py-3 font-medium">{common_description()}</th>
						<th class="px-4 py-3"></th>
					</tr>
				</thead>
				<tbody>
					{#each entries as entry (entry.kind + entry.id)}
						<tr class="border-b border-gray-800 last:border-0 hover:bg-gray-800/40">
							<td class="px-4 py-3">
								<div class="flex items-center gap-2">
									<span class="text-primary">{entry.name}</span>
									{#if entry.kind === 'built_in'}
										<Tag label={common_builtin()} color="Gray" />
									{:else}
										<Tag label={common_custom()} color="Purple" />
									{/if}
								</div>
							</td>
							<td class="text-secondary px-4 py-3">
								{serviceCategories.getName(entry.category) || entry.category}
							</td>
							<td class="text-secondary px-4 py-3">{entry.description}</td>
							<td class="px-4 py-3">
								<div class="flex items-center justify-end gap-3">
									{#if entry.logo_url}
										<img src={entry.logo_url} alt={entry.name} class="h-6 w-6 object-contain" />
									{/if}
									{#if entry.kind === 'custom' && canManage}
										<div class="flex items-center gap-1">
											<button
												class="btn-ghost p-1"
												title={common_edit()}
												onclick={() => handleEdit(entry)}
											>
												<Edit class="h-4 w-4" />
											</button>
											<button
												class="btn-ghost p-1 text-red-400"
												title={common_delete()}
												onclick={() => handleDelete(entry)}
											>
												<Trash2 class="h-4 w-4" />
											</button>
										</div>
									{/if}
								</div>
							</td>
						</tr>
					{:else}
						<tr>
							<td colspan="4" class="px-4 py-10 text-center text-secondary">
								{serviceDefinitions_emptySubtitle()}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}

	<GenericModal
		title={editing ? serviceDefinitions_editCustom() : serviceDefinitions_newCustom()}
		{isOpen}
		onClose={handleClose}
	>
		<div class="space-y-4">
			<TextInput
				label={common_name()}
				field={fieldFor(name, (v) => (name = v))}
				id="custom-service-name"
				helpText={serviceDefinitions_nameHelp()}
			/>
			<TextArea
				label={common_description()}
				field={fieldFor(description, (v) => (description = v))}
				id="custom-service-description"
			/>
			<SelectInput
				label={common_category()}
				field={fieldFor(category, (v) => (category = v))}
				id="custom-service-category"
				options={categoryOptions}
				required={true}
			/>
			<TextInput
				label={serviceDefinitions_logoUrl()}
				field={fieldFor(logoUrl, (v) => (logoUrl = v))}
				id="custom-service-logo"
				helpText={serviceDefinitions_logoUrlHelp()}
			/>
			<Checkbox
				label={serviceDefinitions_logoNeedsWhiteBackground()}
				field={fieldFor(logoNeedsWhiteBackground, (v) => (logoNeedsWhiteBackground = v))}
				id="custom-service-logo-white"
			/>
			<Checkbox
				label={serviceDefinitions_isGeneric()}
				helpText={serviceDefinitions_isGenericHelp()}
				field={fieldFor(isGeneric, (v) => (isGeneric = v))}
				id="custom-service-generic"
			/>
		</div>
		{#snippet footer()}
			<div class="flex justify-end gap-2">
				<button class="btn-secondary" onclick={handleClose}>{common_cancel()}</button>
				<button class="btn-primary" onclick={handleSave}>{common_save()}</button>
			</div>
		{/snippet}
	</GenericModal>
</div>

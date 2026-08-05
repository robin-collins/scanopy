<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required, max } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import type { Category } from '../types/base';
	import { createDefaultCategory } from '../types/base';
	import {
		createColorHelper,
		createIconComponent,
		AVAILABLE_COLORS
	} from '$lib/shared/utils/styling';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { pushError } from '$lib/shared/stores/feedback';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import TextArea from '$lib/shared/components/forms/input/TextArea.svelte';
	import Checkbox from '$lib/shared/components/forms/input/Checkbox.svelte';
	import {
		common_cancel,
		common_color,
		common_couldNotLoadOrganization,
		common_create,
		common_delete,
		common_deleting,
		common_description,
		common_details,
		common_editName,
		common_icon,
		common_name,
		common_saving,
		common_update,
		categories_categoryNamePlaceholder,
		categories_createCategory,
		categories_descriptionPlaceholder,
		categories_iconHelp,
		categories_iconPlaceholder,
		categories_preferredPorts,
		categories_preferredPortsHelp,
		categories_preferredPortsPlaceholder,
		categories_skipFullPortScan,
		categories_skipFullPortScanHelp
	} from '$lib/paraglide/messages';

	let {
		category = null,
		isOpen = false,
		onCreate,
		onUpdate,
		onClose,
		onDelete = null,
		name = undefined
	}: {
		category?: Category | null;
		isOpen?: boolean;
		onCreate: (data: Category) => Promise<void> | void;
		onUpdate: (id: string, data: Category) => Promise<void> | void;
		onClose: () => void;
		onDelete?: ((id: string) => Promise<void> | void) | null;
		name?: string;
	} = $props();

	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	let loading = $state(false);
	let deleting = $state(false);

	let isEditing = $derived(category !== null);
	// Built-in (organization_id null) categories are read-only — the server
	// rejects edits/deletes with 400, so the modal disables the affordance too.
	let isBuiltin = $derived(isEditing && !category?.organization_id);
	let title = $derived(
		isEditing ? common_editName({ name: category?.name ?? '' }) : categories_createCategory()
	);
	let saveLabel = $derived(isEditing ? common_update() : common_create());

	// preferred_ports is a comma-separated string in the UI, Vec<u16> | null on the wire.
	let preferredPortsText = $state('');

	function getDefaultValues(): Category {
		if (category) return { ...category };
		if (organization) return createDefaultCategory(organization.id);
		return createDefaultCategory('');
	}

	function parsePreferredPorts(text: string): number[] | null {
		const ports = text
			.split(',')
			.map((p) => p.trim())
			.filter((p) => p.length > 0)
			.map((p) => Number(p))
			.filter((p) => Number.isInteger(p) && p > 0 && p <= 65535);
		return ports.length > 0 ? ports : null;
	}

	const form = createForm(() => ({
		defaultValues: createDefaultCategory(''),
		onSubmit: async ({ value }) => {
			if (!organization) {
				pushError(common_couldNotLoadOrganization());
				onClose();
				return;
			}

			const categoryData: Category = {
				...(value as Category),
				name: value.name.trim(),
				description: value.description?.trim() || null,
				preferred_ports: parsePreferredPorts(preferredPortsText),
				organization_id: isEditing ? (category?.organization_id ?? null) : organization.id
			};

			loading = true;
			try {
				if (isEditing && category) {
					await onUpdate(category.id, categoryData);
				} else {
					await onCreate(categoryData);
				}
			} finally {
				loading = false;
			}
		}
	}));

	function handleOpen() {
		const defaults = getDefaultValues();
		form.reset(defaults);
		preferredPortsText = defaults.preferred_ports?.join(', ') ?? '';
	}

	async function handleSubmit() {
		await submitForm(form);
	}

	async function handleDelete() {
		if (onDelete && category) {
			deleting = true;
			try {
				await onDelete(category.id);
			} finally {
				deleting = false;
			}
		}
	}

	let colorHelper = $derived(createColorHelper(form.state.values.color));
	let iconPreview = $derived(createIconComponent(form.state.values.icon));
</script>

<GenericModal
	{isOpen}
	{title}
	{name}
	entityId={category?.id}
	size="xl"
	{onClose}
	onOpen={handleOpen}
	showCloseButton={true}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={iconPreview} color={colorHelper.color} />
	{/snippet}

	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			handleSubmit();
		}}
		class="flex min-h-0 flex-1 flex-col"
	>
		<div class="min-h-0 flex-1 overflow-auto p-6">
			<div class="space-y-8">
				<div class="space-y-4">
					<h3 class="text-primary text-lg font-medium">{common_details()}</h3>

					<form.Field
						name="name"
						validators={{
							onBlur: ({ value }) => required(value) || max(100)(value)
						}}
					>
						{#snippet children(field)}
							<TextInput
								label={common_name()}
								id="name"
								{field}
								placeholder={categories_categoryNamePlaceholder()}
								required
								disabled={isBuiltin}
							/>
						{/snippet}
					</form.Field>

					<form.Field
						name="description"
						validators={{
							onBlur: ({ value }) => max(500)(value || '')
						}}
					>
						{#snippet children(field)}
							<TextArea
								label={common_description()}
								id="description"
								{field}
								placeholder={categories_descriptionPlaceholder()}
								disabled={isBuiltin}
							/>
						{/snippet}
					</form.Field>

					<form.Field name="icon">
						{#snippet children(field)}
							<TextInput
								label={common_icon()}
								id="icon"
								{field}
								placeholder={categories_iconPlaceholder()}
								helpText={categories_iconHelp()}
								disabled={isBuiltin}
							/>
						{/snippet}
					</form.Field>

					<form.Field name="color">
						{#snippet children(field)}
							<div class="space-y-2">
								<div class="text-secondary block text-sm font-medium">{common_color()}</div>
								<div class="flex flex-wrap gap-1.5">
									{#each AVAILABLE_COLORS as color (color)}
										{@const ch = createColorHelper(color)}
										<button
											type="button"
											disabled={isBuiltin}
											onclick={() => field.handleChange(color)}
											class="group relative h-7 w-7 rounded-md border-2 transition-all hover:scale-110 disabled:cursor-not-allowed disabled:opacity-50"
											class:border-gray-500={field.state.value !== color}
											class:border-white={field.state.value === color}
											class:ring-2={field.state.value === color}
											class:ring-white={field.state.value === color}
											style="background-color: {ch.rgb};"
											title={color}
										></button>
									{/each}
								</div>
							</div>
						{/snippet}
					</form.Field>
				</div>

				<div class="space-y-4">
					<h3 class="text-primary text-lg font-medium">{categories_skipFullPortScan()}</h3>
					<p class="text-tertiary text-xs">{categories_skipFullPortScanHelp()}</p>

					<form.Field name="skip_full_port_scan">
						{#snippet children(field)}
							<Checkbox label={categories_skipFullPortScan()} {field} id="skip_full_port_scan" />
						{/snippet}
					</form.Field>

					<div>
						<label for="preferred_ports" class="text-secondary mb-2 block text-sm font-medium">
							{categories_preferredPorts()}
						</label>
						<input
							id="preferred_ports"
							type="text"
							bind:value={preferredPortsText}
							placeholder={categories_preferredPortsPlaceholder()}
							class="input-field"
						/>
						<p class="text-tertiary mt-2 text-xs">{categories_preferredPortsHelp()}</p>
					</div>
				</div>
			</div>
		</div>

		{#if isEditing && category}
			<EntityMetadataSection entities={[category]} />
		{/if}

		<div class="modal-footer">
			<div class="flex items-center justify-between">
				<div>
					{#if isEditing && onDelete && !isBuiltin}
						<button
							type="button"
							disabled={deleting || loading}
							onclick={handleDelete}
							class="btn-danger"
						>
							{deleting ? common_deleting() : common_delete()}
						</button>
					{/if}
				</div>
				<div class="flex items-center gap-3">
					<button
						type="button"
						disabled={loading || deleting}
						onclick={onClose}
						class="btn-secondary"
					>
						{common_cancel()}
					</button>
					{#if !isBuiltin}
						<button type="submit" disabled={loading || deleting} class="btn-primary">
							{loading ? common_saving() : saveLabel}
						</button>
					{/if}
				</div>
			</div>
		</div>
	</form>
</GenericModal>

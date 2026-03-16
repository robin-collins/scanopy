<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required, max } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import type { Credential, CredentialType } from '../types/base';
	import { createDefaultCredential } from '../types/base';
	import { entities, credentialTypes } from '$lib/shared/stores/metadata';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { pushError } from '$lib/shared/stores/feedback';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import TagPicker from '$lib/features/tags/components/TagPicker.svelte';
	import type { FieldDefinition } from '$lib/shared/stores/metadata';
	import {
		common_cancel,
		common_couldNotLoadOrganization,
		common_create,
		common_delete,
		common_deleting,
		common_details,
		common_editName,
		common_name,
		common_saving,
		common_update
	} from '$lib/paraglide/messages';

	let {
		credential = null,
		isOpen = false,
		onCreate,
		onUpdate,
		onClose,
		onDelete = null,
		name = undefined
	}: {
		credential?: Credential | null;
		isOpen?: boolean;
		onCreate: (data: Credential) => Promise<void> | void;
		onUpdate: (id: string, data: Credential) => Promise<void> | void;
		onClose: () => void;
		onDelete?: ((id: string) => Promise<void> | void) | null;
		name?: string;
	} = $props();

	// TanStack Query for organization
	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	let loading = $state(false);
	let deleting = $state(false);

	let isEditing = $derived(credential !== null);
	let title = $derived(
		isEditing ? common_editName({ name: credential?.name ?? '' }) : 'Create Credential'
	);
	let saveLabel = $derived(isEditing ? common_update() : common_create());

	// Selected credential type ID for dynamic form rendering
	let selectedTypeId = $state<string>('Snmp');

	// Dynamic field values keyed by field ID
	let fieldValues = $state<Record<string, string>>({});

	function getDefaultValues(): Credential {
		if (credential) return { ...credential };
		if (organization) return createDefaultCredential(organization.id);
		return createDefaultCredential('');
	}

	let colorHelper = $derived(entities.getColorHelper('Credential'));

	// Get field definitions for the currently selected type
	let currentFields: FieldDefinition[] = $derived.by(() => {
		const meta = credentialTypes.getMetadata(selectedTypeId);
		return meta?.fields ?? [];
	});

	// Create form
	const form = createForm(() => ({
		defaultValues: createDefaultCredential(''),
		onSubmit: async ({ value }) => {
			if (!organization) {
				pushError(common_couldNotLoadOrganization());
				onClose();
				return;
			}

			// Build credential_type from fieldValues and selectedTypeId
			const credentialType = buildCredentialType();

			const credentialData: Credential = {
				...(value as Credential),
				name: value.name.trim(),
				organization_id: organization.id,
				credential_type: credentialType
			};

			loading = true;
			try {
				if (isEditing && credential) {
					await onUpdate(credential.id, credentialData);
				} else {
					await onCreate(credentialData);
				}
			} finally {
				loading = false;
			}
		}
	}));

	/**
	 * Build a CredentialType object from the current selectedTypeId and fieldValues.
	 */
	function buildCredentialType(): CredentialType {
		const fields = currentFields;
		const typeObj: Record<string, unknown> = { type: selectedTypeId };

		for (const field of fields) {
			const value = fieldValues[field.id];
			if (field.optional && (!value || value.trim() === '')) {
				typeObj[field.id] = null;
			} else {
				typeObj[field.id] = value ?? (field.default_value || '');
			}
		}

		return typeObj as unknown as CredentialType;
	}

	/**
	 * Initialize fieldValues from a credential's credential_type
	 */
	function initFieldValues(ct: CredentialType) {
		const values: Record<string, string> = {};
		const raw = ct as unknown as Record<string, unknown>;
		for (const [key, val] of Object.entries(raw)) {
			if (key === 'type') continue;
			values[key] = val != null ? String(val) : '';
		}
		fieldValues = values;
	}

	/**
	 * Initialize fieldValues with defaults for a given type
	 */
	function initDefaultFieldValues(typeId: string) {
		const meta = credentialTypes.getMetadata(typeId);
		const fields: FieldDefinition[] = meta?.fields ?? [];
		const values: Record<string, string> = {};
		for (const field of fields) {
			values[field.id] = field.default_value ?? '';
		}
		fieldValues = values;
	}

	// Reset form when modal opens
	function handleOpen() {
		const defaults = getDefaultValues();
		form.reset(defaults);

		if (credential) {
			selectedTypeId = credential.credential_type.type;
			initFieldValues(credential.credential_type);
		} else {
			selectedTypeId = 'Snmp';
			initDefaultFieldValues('Snmp');
		}
	}

	function handleTypeChange(event: Event) {
		const select = event.target as HTMLSelectElement;
		selectedTypeId = select.value;
		initDefaultFieldValues(selectedTypeId);
	}

	async function handleSubmit() {
		await submitForm(form);
	}

	async function handleDelete() {
		if (onDelete && credential) {
			deleting = true;
			try {
				await onDelete(credential.id);
			} finally {
				deleting = false;
			}
		}
	}

	let typeOptions = $derived(credentialTypes.getItems());
</script>

<GenericModal
	{isOpen}
	{title}
	{name}
	entityId={credential?.id}
	size="xl"
	{onClose}
	onOpen={handleOpen}
	showCloseButton={true}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={entities.getIconComponent('Credential')} color={colorHelper.color} />
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
				<!-- Credential Details Section -->
				<div class="space-y-4">
					<p class="text-secondary">
						Create credentials to authenticate with network devices and services.
					</p>
					<h3 class="text-primary flex items-center gap-2 text-lg font-medium">
						{common_details()}
					</h3>

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
								placeholder="e.g. Office SNMP"
								required
							/>
						{/snippet}
					</form.Field>

					<!-- Type Selector (only on create) -->
					<div class="space-y-2">
						<label for="credential_type" class="text-secondary block text-sm font-medium">
							Credential Type
						</label>
						<select
							id="credential_type"
							value={selectedTypeId}
							onchange={handleTypeChange}
							disabled={isEditing}
							class="select-trigger text-primary w-full rounded-md px-3 py-2 text-sm"
						>
							{#each typeOptions as typeOption (typeOption.id)}
								<option value={typeOption.id}>{typeOption.name}</option>
							{/each}
						</select>
						{#if isEditing}
							<p class="text-muted text-xs">Type cannot be changed after creation.</p>
						{/if}
					</div>

					<!-- Dynamic Fields from Fixture -->
					{#each currentFields as field (field.id)}
						<div class="space-y-1">
							<label for={field.id} class="text-secondary block text-sm font-medium">
								{field.label}
								{#if !field.optional}
									<span class="text-red-400">*</span>
								{/if}
							</label>

							{#if field.field_type === 'select'}
								<select
									id={field.id}
									value={fieldValues[field.id] ?? field.default_value ?? ''}
									onchange={(e) => {
										const target = e.target as HTMLSelectElement;
										fieldValues[field.id] = target.value;
									}}
									class="select-trigger text-primary w-full rounded-md px-3 py-2 text-sm"
								>
									{#each field.options ?? [] as option (option)}
										<option value={option}>{option}</option>
									{/each}
								</select>
							{:else if field.field_type === 'text'}
								<textarea
									id={field.id}
									value={fieldValues[field.id] ?? ''}
									oninput={(e) => {
										const target = e.target as HTMLTextAreaElement;
										fieldValues[field.id] = target.value;
									}}
									placeholder={field.placeholder ?? ''}
									rows={4}
									class="input-field text-primary w-full rounded-md px-3 py-2 font-mono text-sm"
									class:password-field={field.secret}
								></textarea>
							{:else}
								<input
									id={field.id}
									type={field.secret ? 'password' : 'text'}
									value={fieldValues[field.id] ?? ''}
									oninput={(e) => {
										const target = e.target as HTMLInputElement;
										fieldValues[field.id] = target.value;
									}}
									placeholder={field.placeholder ?? ''}
									required={!field.optional}
									class="input-field text-primary w-full rounded-md px-3 py-2 text-sm"
								/>
							{/if}

							{#if field.help_text}
								<p class="text-muted text-xs">{field.help_text}</p>
							{/if}
						</div>
					{/each}

					<form.Field name="tags">
						{#snippet children(field)}
							<TagPicker
								selectedTagIds={field.state.value || []}
								onChange={(tags) => field.handleChange(tags)}
							/>
						{/snippet}
					</form.Field>
				</div>
			</div>
		</div>

		{#if isEditing && credential}
			<EntityMetadataSection entities={[credential]} />
		{/if}

		<!-- Footer -->
		<div class="modal-footer">
			<div class="flex items-center justify-between">
				<div>
					{#if isEditing && onDelete}
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
					<button type="submit" disabled={loading || deleting} class="btn-primary">
						{loading ? common_saving() : saveLabel}
					</button>
				</div>
			</div>
		</div>
	</form>
</GenericModal>

<style>
	.password-field {
		-webkit-text-security: disc;
	}
</style>

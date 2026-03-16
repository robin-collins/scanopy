<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required, max } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { entities, credentialTypes } from '$lib/shared/stores/metadata';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import type { Network } from '../types';
	import { createEmptyNetworkFormData } from '../queries';
	import { pushError } from '$lib/shared/stores/feedback';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import TagPicker from '$lib/features/tags/components/TagPicker.svelte';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import { getCredentialTypeId } from '$lib/features/credentials/types/base';
	import {
		common_cancel,
		common_couldNotLoadUser,
		common_create,
		common_delete,
		common_deleting,
		common_details,
		common_editName,
		common_name,
		common_saving,
		common_update,
		networks_createNetwork,
		networks_networkNamePlaceholder
	} from '$lib/paraglide/messages';

	let {
		network = null,
		isOpen = false,
		onCreate,
		onUpdate,
		onClose,
		onDelete = null,
		name = undefined
	}: {
		network?: Network | null;
		isOpen?: boolean;
		onCreate: (data: Network) => Promise<void> | void;
		onUpdate: (id: string, data: Network) => Promise<void> | void;
		onClose: () => void;
		onDelete?: ((id: string) => Promise<void> | void) | null;
		name?: string;
	} = $props();

	// TanStack Query for organization and current user
	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	// Demo mode check
	let isDemoOrg = $derived(organization?.plan?.type === 'Demo');
	let isNonOwnerInDemo = $derived(isDemoOrg && currentUser?.permissions !== 'Owner');

	// TanStack Query for credentials
	const credentialsQuery = useCredentialsQuery();
	let allCredentials = $derived(credentialsQuery.data ?? []);

	let loading = $state(false);
	let deleting = $state(false);

	let isEditing = $derived(network !== null);
	let title = $derived(
		isEditing ? common_editName({ name: network?.name ?? '' }) : networks_createNetwork()
	);
	let saveLabel = $derived(isEditing ? common_update() : common_create());

	// Local state for selected credential IDs
	let selectedCredentialIds = $state<string[]>([]);

	function getDefaultValues() {
		return network
			? { ...network, seedData: false }
			: { ...createEmptyNetworkFormData(), seedData: true };
	}

	// Create form
	const form = createForm(() => ({
		defaultValues: {
			...createEmptyNetworkFormData(),
			seedData: true
		},
		onSubmit: async ({ value }) => {
			if (!organization) {
				pushError(common_couldNotLoadUser());
				handleClose();
				return;
			}

			const networkData: Network = {
				...(value as Network),
				name: value.name.trim(),
				organization_id: organization.id,
				credential_ids: selectedCredentialIds
			};

			loading = true;
			try {
				if (isEditing && network) {
					await onUpdate(network.id, networkData);
				} else {
					await onCreate(networkData);
				}
			} finally {
				loading = false;
			}
		}
	}));

	// Reset form when modal opens
	function handleOpen() {
		const defaults = getDefaultValues();
		selectedCredentialIds = defaults.credential_ids ?? [];

		form.reset({
			...defaults
		});
	}

	function handleClose() {
		onClose();
	}

	async function handleSubmit() {
		await submitForm(form);
	}

	async function handleDelete() {
		if (onDelete && network) {
			deleting = true;
			try {
				await onDelete(network.id);
			} finally {
				deleting = false;
			}
		}
	}

	function toggleCredential(credentialId: string) {
		if (selectedCredentialIds.includes(credentialId)) {
			selectedCredentialIds = selectedCredentialIds.filter((id) => id !== credentialId);
		} else {
			selectedCredentialIds = [...selectedCredentialIds, credentialId];
		}
	}

	let colorHelper = entities.getColorHelper('Network');
</script>

<GenericModal
	{isOpen}
	{title}
	{name}
	entityId={network?.id}
	size="xl"
	onClose={handleClose}
	onOpen={handleOpen}
	showCloseButton={true}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={entities.getIconComponent('Network')} color={colorHelper.color} />
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
				<!-- Network Details Section -->
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
								placeholder={networks_networkNamePlaceholder()}
								required
							/>
						{/snippet}
					</form.Field>

					<form.Field name="tags">
						{#snippet children(field)}
							<TagPicker
								selectedTagIds={field.state.value || []}
								onChange={(tags) => field.handleChange(tags)}
							/>
						{/snippet}
					</form.Field>

					<!-- Credentials Multi-Select -->
					<div class="space-y-2">
						<!-- svelte-ignore a11y_label_has_associated_control -->
						<label class="text-secondary block text-sm font-medium"> Credentials </label>
						{#if allCredentials.length === 0}
							<p class="text-muted text-sm">
								No credentials available. Create credentials in the Credentials tab.
							</p>
						{:else}
							<div
								class="border-secondary max-h-48 space-y-1 overflow-y-auto rounded-md border p-2"
							>
								{#each allCredentials as cred (cred.id)}
									{@const typeId = getCredentialTypeId(cred)}
									<label
										class="flex cursor-pointer items-center gap-3 rounded px-2 py-1.5 hover:bg-white/5"
										class:opacity-50={isNonOwnerInDemo}
									>
										<input
											type="checkbox"
											checked={selectedCredentialIds.includes(cred.id)}
											onchange={() => toggleCredential(cred.id)}
											disabled={isNonOwnerInDemo}
											class="rounded"
										/>
										<span class="text-primary text-sm">{cred.name}</span>
										<span
											class="rounded px-1.5 py-0.5 text-xs"
											style="background-color: {credentialTypes.getColorHelper(typeId)
												.color}20; color: {credentialTypes.getColorHelper(typeId).color}"
										>
											{credentialTypes.getName(typeId)}
										</span>
									</label>
								{/each}
							</div>
						{/if}
						<p class="text-muted mt-1 text-xs">
							{#if isNonOwnerInDemo}
								Credential settings are read-only in demo mode.
							{:else}
								Select credentials to use for discovery on this network. SNMP always tries "public"
								as fallback. Hosts can override.
							{/if}
						</p>
					</div>
				</div>
			</div>
		</div>

		{#if isEditing && network}
			<EntityMetadataSection entities={[network]} />
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
						onclick={handleClose}
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

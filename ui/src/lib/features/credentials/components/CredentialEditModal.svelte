<script lang="ts">
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import type { Credential } from '../types/base';
	import { entities } from '$lib/shared/stores/metadata';
	import CredentialForm from './CredentialForm.svelte';
	import {
		common_editName,
		credentials_createCredential,
		credentials_description
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

	let isEditing = $derived(credential !== null);
	let title = $derived(
		isEditing ? common_editName({ name: credential?.name ?? '' }) : credentials_createCredential()
	);

	let colorHelper = $derived(entities.getColorHelper('Credential'));

	let credentialFormRef: ReturnType<typeof CredentialForm> | undefined = $state();

	function handleOpen() {
		credentialFormRef?.reset();
	}

	async function handleSave(data: Credential) {
		if (isEditing && credential) {
			await onUpdate(credential.id, data);
		} else {
			await onCreate(data);
		}
	}

	async function handleDelete(id: string) {
		if (onDelete) {
			await onDelete(id);
		}
	}
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

	<div class="min-h-0 flex-1 overflow-auto p-6">
		<div class="space-y-4">
			<p class="text-secondary text-sm">
				{credentials_description()}
			</p>

			<CredentialForm
				bind:this={credentialFormRef}
				{credential}
				onSave={handleSave}
				onDelete={onDelete ? handleDelete : null}
			/>
		</div>
	</div>

	{#if isEditing && credential}
		<EntityMetadataSection entities={[credential]} />
	{/if}
</GenericModal>

<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { validateForm } from '$lib/shared/components/forms/form-context';
	import ListConfigEditor from '$lib/shared/components/forms/selection/ListConfigEditor.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import { CredentialTypeDisplay } from '$lib/shared/components/forms/selection/display/CredentialTypeDisplay.svelte';
	import { CredentialDisplay } from '$lib/shared/components/forms/selection/display/CredentialDisplay.svelte';
	import CredentialForm from '$lib/features/credentials/components/CredentialForm.svelte';
	import EntityConfigEmpty from '$lib/shared/components/forms/EntityConfigEmpty.svelte';
	import EntityTag from '$lib/shared/components/data/EntityTag.svelte';
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import type { Credential, CredentialType } from '$lib/features/credentials/types/base';
	import { createDefaultCredential } from '$lib/features/credentials/types/base';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import { v4 as uuidv4 } from 'uuid';
	import {
		daemons_credentialWizardTitle,
		daemons_credentialWizardDescription,
		daemons_credentialWizardSelectType,
		daemons_credentialWizardEmpty,
		daemons_credentialWizardNetworkCredentials
	} from '$lib/paraglide/messages';

	export interface PendingCredential {
		credential: Credential;
		seedIp: string;
		fieldValues: Record<string, string>;
	}

	interface Props {
		daemonName?: string;
		networkId?: string;
		pendingCredentials: PendingCredential[];
		onRemoveCredential?: (credential: Credential) => void;
	}

	let {
		daemonName = 'scanopy-daemon',
		networkId = '',
		pendingCredentials = $bindable([]),
		onRemoveCredential
	}: Props = $props();

	// Query network and credential data for network-level credential display
	const networksQuery = useNetworksQuery();
	const credentialsQuery = useCredentialsQuery();

	let networkCredentials = $derived.by(() => {
		if (!networkId || !networksQuery.data || !credentialsQuery.data) return [];
		const network = networksQuery.data.find((n) => n.id === networkId);
		if (!network?.credential_ids?.length) return [];
		return credentialsQuery.data.filter((c) => network.credential_ids!.includes(c.id));
	});

	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	// Local items array for ListConfigEditor display
	let items = $derived(pendingCredentials.map((p) => p.credential));

	let typeOptions = $derived(credentialTypes.getItems());

	// Refs to each CredentialForm for buildCredentialType()
	let credentialFormRefs: (ReturnType<typeof CredentialForm> | undefined)[] = $state([]);

	// Build form default values from pendingCredentials
	function buildFormDefaults() {
		const credentials: Record<string, unknown>[] = pendingCredentials.map((p) => ({
			seedIp: p.seedIp,
			fields: { ...p.fieldValues }
		}));
		return { credentials };
	}

	// TanStack form owns all credential field data
	const form = createForm(() => ({
		defaultValues: buildFormDefaults(),
		onSubmit: async () => {
			// Handled externally via validate()
		}
	}));

	function syncFormDefaults() {
		form.reset(buildFormDefaults());
	}

	function initDefaultFieldValues(typeId: string): Record<string, string> {
		const meta = credentialTypes.getMetadata(typeId);
		const fields = meta?.fields ?? [];
		const values: Record<string, string> = {};
		for (const field of fields) {
			if (field.field_type === 'pathorinline') {
				values[field.id] = JSON.stringify({ mode: 'Inline', value: '' });
			} else {
				values[field.id] = field.default_value ?? '';
			}
		}
		return values;
	}

	function handleAddCredential(typeId: string) {
		if (!organization) return;

		const cred = {
			...createDefaultCredential(organization.id),
			id: uuidv4(),
			name: credentialTypes.getName(typeId),
			credential_type: { type: typeId } as Credential['credential_type']
		};

		// Set defaults from fixture metadata
		const meta = credentialTypes.getMetadata(typeId);
		if (meta?.fields) {
			const ct = cred.credential_type as unknown as Record<string, unknown>;
			for (const field of meta.fields) {
				if (field.default_value != null && ct[field.id] === undefined) {
					if (field.field_type === 'secretpathorinline' || field.field_type === 'pathorinline') {
						ct[field.id] = { mode: 'Inline', value: field.default_value };
					} else {
						const num = Number(field.default_value);
						ct[field.id] = !isNaN(num) ? num : field.default_value;
					}
				}
			}
		}

		const fieldValues = initDefaultFieldValues(typeId);
		pendingCredentials = [...pendingCredentials, { credential: cred, seedIp: '', fieldValues }];
		syncFormDefaults();
	}

	function handleRemoveCredential(index: number) {
		const removed = pendingCredentials[index];
		if (removed) {
			onRemoveCredential?.(removed.credential);
		}
		pendingCredentials = pendingCredentials.filter((_, i) => i !== index);
		syncFormDefaults();
	}

	function handleCredentialChange(credential: Credential, index: number) {
		pendingCredentials = pendingCredentials.map((p, i) => (i === index ? { ...p, credential } : p));
	}

	function handleConfigChange(
		index: number,
		data: { seedIp?: string; fieldValues?: Record<string, string> }
	) {
		pendingCredentials = pendingCredentials.map((p, i) => {
			if (i !== index) return p;
			const updated = { ...p };
			if (data.seedIp !== undefined) {
				updated.seedIp = data.seedIp;
				// Update credential name based on seedIp
				const ip = data.seedIp.trim();
				const isLocalhost = ip === '127.0.0.1' || ip === '::1' || ip === 'localhost' || ip === '';
				const name = isLocalhost ? daemonName : ip;
				updated.credential = { ...p.credential, name };
			}
			if (data.fieldValues !== undefined) {
				updated.fieldValues = data.fieldValues;
			}
			return updated;
		});
	}

	/** Validate all fields across all credentials. Returns true if valid. */
	export async function validate(): Promise<boolean> {
		const isValid = await validateForm(form);
		return isValid;
	}

	/** Get credentials ready for bulk creation (with built credential_type from fieldValues). */
	export function getCredentialsForCreate(): { credential: Credential; seedIp: string }[] {
		return pendingCredentials.map((p, i) => {
			const ref = credentialFormRefs[i];
			const credentialType =
				ref?.buildCredentialType() ?? (p.credential.credential_type as CredentialType);
			return {
				credential: {
					...p.credential,
					credential_type: credentialType
				},
				seedIp: p.seedIp
			};
		});
	}
</script>

<div class="flex min-h-0 flex-1 flex-col">
	{#if networkCredentials.length > 0}
		<div class="mb-3 flex flex-wrap items-center gap-2">
			<span class="text-secondary text-xs font-medium">
				{daemons_credentialWizardNetworkCredentials()}
			</span>
			{#each networkCredentials as cred (cred.id)}
				<EntityTag
					entityRef={{
						entityType: 'Credential',
						entityId: cred.id,
						data: cred
					}}
					label={cred.name}
					color="Green"
				/>
			{/each}
		</div>
	{/if}
	<ListConfigEditor {items} onChange={handleCredentialChange}>
		<svelte:fragment slot="list" let:items let:onEdit let:highlightedIndex let:onItemSelect>
			<ListManager
				label={daemons_credentialWizardTitle()}
				helpText={daemons_credentialWizardDescription()}
				placeholder={daemons_credentialWizardSelectType()}
				emptyMessage={daemons_credentialWizardEmpty()}
				options={typeOptions}
				itemClickAction="edit"
				allowReorder={false}
				allowDuplicates={true}
				optionDisplayComponent={CredentialTypeDisplay}
				itemDisplayComponent={CredentialDisplay}
				{items}
				onAdd={handleAddCredential}
				onRemove={handleRemoveCredential}
				onClick={onItemSelect}
				{onEdit}
				{highlightedIndex}
			/>
		</svelte:fragment>

		<svelte:fragment slot="config" let:selectedItem let:selectedIndex>
			<!-- Render ALL config panels, hide non-selected (like InterfacesForm) -->
			{#each pendingCredentials as pending, index (`${pending.credential.id}-${index}`)}
				<div class:hidden={selectedIndex !== index}>
					<CredentialForm
						bind:this={credentialFormRefs[index]}
						{form}
						compact={true}
						fieldPrefix={`credentials[${index}].`}
						fixedCredentialType={pending.credential.credential_type.type}
						fixedName={pending.credential.name}
						onChange={(data) => handleConfigChange(index, data)}
					/>
				</div>
			{/each}

			{#if !selectedItem}
				<EntityConfigEmpty title={daemons_credentialWizardSelectType()} subtitle="" />
			{/if}
		</svelte:fragment>
	</ListConfigEditor>
</div>

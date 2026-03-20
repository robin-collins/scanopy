<script lang="ts">
	import ListConfigEditor from '$lib/shared/components/forms/selection/ListConfigEditor.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import { CredentialTypeDisplay } from '$lib/shared/components/forms/selection/display/CredentialTypeDisplay.svelte';
	import { CredentialDisplay } from '$lib/shared/components/forms/selection/display/CredentialDisplay.svelte';
	import CredentialForm from '$lib/features/credentials/components/CredentialForm.svelte';
	import EntityConfigEmpty from '$lib/shared/components/forms/EntityConfigEmpty.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import type { Credential } from '$lib/features/credentials/types/base';
	import { createDefaultCredential } from '$lib/features/credentials/types/base';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { v4 as uuidv4 } from 'uuid';
	import {
		daemons_credentialWizardTitle,
		daemons_credentialWizardDescription,
		daemons_credentialWizardSelectType,
		daemons_credentialWizardEmpty,
		daemons_credentialWizardSeedIp,
		daemons_credentialWizardSeedIpHelp,
		daemons_credentialWizardLocalhostHelp
	} from '$lib/paraglide/messages';

	export interface PendingCredential {
		credential: Credential;
		seedIp: string;
	}

	interface Props {
		daemonName?: string;
		pendingCredentials: PendingCredential[];
	}

	let { daemonName = 'scanopy-daemon', pendingCredentials = $bindable([]) }: Props = $props();

	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	// Local items array for ListConfigEditor
	let items = $derived(pendingCredentials.map((p) => p.credential));

	let typeOptions = $derived(credentialTypes.getItems());

	function handleAddCredential(typeId: string) {
		if (!organization) return;

		const cred = {
			...createDefaultCredential(organization.id),
			id: uuidv4(),
			name: `${daemonName} ${credentialTypes.getName(typeId)}`,
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

		pendingCredentials = [...pendingCredentials, { credential: cred, seedIp: '' }];
	}

	function handleRemoveCredential(index: number) {
		pendingCredentials = pendingCredentials.filter((_, i) => i !== index);
	}

	function handleCredentialChange(credential: Credential, index: number) {
		pendingCredentials = pendingCredentials.map((p, i) => (i === index ? { ...p, credential } : p));
	}

	async function handleFormSave(data: Credential, index: number) {
		data.name = `${daemonName} ${credentialTypes.getName(data.credential_type.type)}`;
		handleCredentialChange(data, index);
	}

	function handleSeedIpChange(index: number, value: string) {
		pendingCredentials = pendingCredentials.map((p, i) =>
			i === index ? { ...p, seedIp: value } : p
		);
	}
</script>

<div class="flex min-h-0 flex-1 flex-col">
	<div class="min-h-0 flex-1">
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
				{#if selectedItem != null && selectedIndex >= 0}
					{@const seedIp = pendingCredentials[selectedIndex]?.seedIp ?? ''}
					<div class="space-y-4">
						<!-- Target IP field -->
						<div class="space-y-1">
							<label for="seed-ip-{selectedIndex}" class="text-secondary block text-sm font-medium">
								{daemons_credentialWizardSeedIp()}
							</label>
							<input
								id="seed-ip-{selectedIndex}"
								type="text"
								value={seedIp}
								disabled={selectedItem.credential_type.type === 'DockerProxy'}
								oninput={(e) => {
									const target = e.target as HTMLInputElement;
									handleSeedIpChange(selectedIndex, target.value);
								}}
								placeholder="e.g. 192.168.1.1"
								class="input-field text-primary w-full rounded-md px-3 py-2 text-sm disabled:opacity-50"
							/>
							<p class="text-muted text-xs">{daemons_credentialWizardSeedIpHelp()}</p>

							{#if seedIp === '127.0.0.1' || seedIp === '::1'}
								<InlineInfo title="" body={daemons_credentialWizardLocalhostHelp()} />
							{/if}
						</div>

						{#key selectedItem.id}
							<CredentialForm
								credential={selectedItem}
								fixedCredentialType={selectedItem.credential_type.type}
								fixedName={selectedItem.name}
								compact={true}
								onSave={(data) => handleFormSave(data, selectedIndex)}
							/>
						{/key}
					</div>
				{:else}
					<EntityConfigEmpty title={daemons_credentialWizardSelectType()} subtitle="" />
				{/if}
			</svelte:fragment>
		</ListConfigEditor>
	</div>
</div>

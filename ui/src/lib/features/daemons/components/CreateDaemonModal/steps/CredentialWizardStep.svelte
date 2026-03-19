<script lang="ts">
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import { CredentialTypeDisplay } from '$lib/shared/components/forms/selection/display/CredentialTypeDisplay.svelte';
	import { CredentialDisplay } from '$lib/shared/components/forms/selection/display/CredentialDisplay.svelte';
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import CredentialForm from '$lib/features/credentials/components/CredentialForm.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import InlineSuccess from '$lib/shared/components/feedback/InlineSuccess.svelte';
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import { useCreateCredentialMutation } from '$lib/features/credentials/queries';
	import { pushSuccess } from '$lib/shared/stores/feedback';
	import { Trash2 } from 'lucide-svelte';
	import type { Credential } from '$lib/features/credentials/types/base';
	import {
		common_create,
		daemons_credentialWizardTitle,
		daemons_credentialWizardDescription,
		daemons_credentialWizardSelectType,
		daemons_credentialWizardEmpty,
		daemons_credentialWizardSeedIp,
		daemons_credentialWizardSeedIpHelp,
		daemons_credentialWizardLocalhostHelp,
		daemons_credentialWizardCreated
	} from '$lib/paraglide/messages';

	interface WizardCredential {
		id: string;
		typeId: string;
		credential: Credential | null;
		seedIp: string;
	}

	interface Props {
		daemonName?: string;
		credentialIds: string[];
		onCredentialCreated?: (id: string) => void;
		onCredentialRemoved?: (id: string) => void;
	}

	let {
		daemonName = 'scanopy-daemon',
		credentialIds = $bindable([]),
		onCredentialCreated,
		onCredentialRemoved
	}: Props = $props();

	const createCredentialMutation = useCreateCredentialMutation();

	let wizardItems = $state<WizardCredential[]>([]);
	let selectedIndex = $state(-1);
	let selectedItem = $derived(selectedIndex >= 0 ? wizardItems[selectedIndex] : null);

	let typeOptions = $derived(credentialTypes.getItems());

	function handleTypeSelect(typeId: string) {
		const newItem: WizardCredential = {
			id: crypto.randomUUID(),
			typeId,
			credential: null,
			seedIp: typeId === 'DockerProxy' ? '127.0.0.1' : ''
		};
		wizardItems = [...wizardItems, newItem];
		selectedIndex = wizardItems.length - 1;
	}

	function handleRemove(index: number) {
		const item = wizardItems[index];
		if (item.credential) {
			credentialIds = credentialIds.filter((id) => id !== item.credential!.id);
			onCredentialRemoved?.(item.credential!.id);
		}
		wizardItems = wizardItems.filter((_, i) => i !== index);
		if (selectedIndex >= wizardItems.length) {
			selectedIndex = wizardItems.length - 1;
		}
	}

	async function handleSave(data: Credential, item: WizardCredential) {
		data.name = `${daemonName} ${credentialTypes.getName(item.typeId)}`;

		// Set seed_ips if provided
		if (item.seedIp.trim()) {
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			(data as any).seed_ips = [item.seedIp.trim()];
		}

		const created = await createCredentialMutation.mutateAsync(data);

		// Update the wizard item with the created credential
		wizardItems = wizardItems.map((w) => (w.id === item.id ? { ...w, credential: created } : w));

		credentialIds = [...credentialIds, created.id];
		onCredentialCreated?.(created.id);
		pushSuccess(daemons_credentialWizardCreated());
	}

	function handleSeedIpChange(itemId: string, value: string) {
		wizardItems = wizardItems.map((w) => (w.id === itemId ? { ...w, seedIp: value } : w));
	}

	let isLocalhostSeedIp = $derived(
		selectedItem?.seedIp === '127.0.0.1' || selectedItem?.seedIp === '::1'
	);
</script>

<div class="space-y-4">
	<div>
		<h3 class="text-primary text-sm font-medium">{daemons_credentialWizardTitle()}</h3>
		<p class="text-muted mt-1 text-xs">{daemons_credentialWizardDescription()}</p>
	</div>

	<div class="flex min-h-0 gap-4">
		<!-- Left panel: list -->
		<div class="w-2/5 space-y-3">
			<RichSelect
				label=""
				selectedValue=""
				options={typeOptions}
				displayComponent={CredentialTypeDisplay}
				onSelect={handleTypeSelect}
				placeholder={daemons_credentialWizardSelectType()}
			/>

			{#if wizardItems.length === 0}
				<p class="text-tertiary py-4 text-center text-xs">{daemons_credentialWizardEmpty()}</p>
			{/if}

			{#each wizardItems as item, index (item.id)}
				<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
				<div
					class="flex w-full cursor-pointer items-center gap-2 rounded-md border p-2 text-left text-sm transition-colors {selectedIndex ===
					index
						? 'border-blue-500 bg-blue-900/20'
						: 'border-gray-600 hover:border-gray-500'}"
					onclick={() => (selectedIndex = index)}
				>
					<div class="min-w-0 flex-1">
						{#if item.credential}
							<ListSelectItem
								item={item.credential}
								context={{ seed_ips: item.seedIp ? [item.seedIp] : [] }}
								displayComponent={CredentialDisplay}
							/>
						{:else}
							<div class="flex items-center gap-2">
								<span class="text-secondary text-sm">
									{credentialTypes.getName(item.typeId)}
								</span>
								<span class="text-tertiary text-xs italic">
									{daemons_credentialWizardEmpty()}
								</span>
							</div>
						{/if}
					</div>
					<button
						type="button"
						class="text-muted hover:text-danger shrink-0 p-1"
						onclick={(e) => {
							e.stopPropagation();
							handleRemove(index);
						}}
					>
						<Trash2 class="h-3.5 w-3.5" />
					</button>
				</div>
			{/each}
		</div>

		<!-- Right panel: config -->
		<div class="w-3/5 border-l border-gray-600 pl-4">
			{#if selectedItem && !selectedItem.credential}
				<div class="space-y-4">
					<CredentialForm
						fixedCredentialType={selectedItem.typeId}
						fixedName={`${daemonName} ${credentialTypes.getName(selectedItem.typeId)}`}
						saveLabel={common_create()}
						compact={true}
						onSave={(data) => handleSave(data, selectedItem!)}
					/>

					<!-- Seed IP field -->
					<div class="space-y-1">
						<label for="seed-ip" class="text-secondary block text-sm font-medium">
							{daemons_credentialWizardSeedIp()}
						</label>
						<input
							id="seed-ip"
							type="text"
							value={selectedItem.seedIp}
							disabled={selectedItem.typeId === 'DockerProxy'}
							oninput={(e) => {
								const target = e.target as HTMLInputElement;
								handleSeedIpChange(selectedItem!.id, target.value);
							}}
							placeholder="e.g. 192.168.1.1"
							class="input-field text-primary w-full rounded-md px-3 py-2 text-sm disabled:opacity-50"
						/>
						<p class="text-muted text-xs">{daemons_credentialWizardSeedIpHelp()}</p>

						{#if isLocalhostSeedIp}
							<InlineInfo title="" body={daemons_credentialWizardLocalhostHelp()} />
						{/if}
					</div>
				</div>
			{:else if selectedItem?.credential}
				<InlineSuccess title={daemons_credentialWizardCreated()} />
				<div class="mt-3">
					<ListSelectItem
						item={selectedItem.credential}
						context={{}}
						displayComponent={CredentialDisplay}
					/>
				</div>
			{:else}
				<div class="text-tertiary flex h-32 items-center justify-center">
					<p class="text-sm">{daemons_credentialWizardSelectType()}</p>
				</div>
			{/if}
		</div>
	</div>
</div>

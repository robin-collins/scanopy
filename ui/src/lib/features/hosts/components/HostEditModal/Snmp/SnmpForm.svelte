<script lang="ts">
	import type { HostFormData } from '$lib/features/hosts/types/base';
	import type { Network } from '$lib/features/networks/types';
	import type { Credential } from '$lib/features/credentials/types/base';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import ListConfigEditor from '$lib/shared/components/forms/selection/ListConfigEditor.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import EntityConfigEmpty from '$lib/shared/components/forms/EntityConfigEmpty.svelte';
	import ConfigHeader from '$lib/shared/components/forms/config/ConfigHeader.svelte';
	import { CredentialDisplay } from '$lib/shared/components/forms/selection/display/CredentialDisplay.svelte';

	interface Props {
		formData: HostFormData;
		network?: Network | null;
	}

	let { formData = $bindable(), network = null }: Props = $props();

	// TanStack Query for organization and current user (for demo mode check)
	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	// Demo mode check: only Owner can modify credential settings in demo orgs
	let isDemoOrg = $derived(organization?.plan?.type === 'Demo');
	let isNonOwnerInDemo = $derived(isDemoOrg && currentUser?.permissions !== 'Owner');

	// TanStack Query for credentials
	const credentialsQuery = useCredentialsQuery();
	let allCredentials = $derived(credentialsQuery.data ?? []);

	// Resolve credential assignments to full credential objects for list display
	let selectedCredentials = $derived(
		(formData.credential_assignments ?? [])
			.map((a) => allCredentials.find((c) => c.id === a.credential_id))
			.filter((c): c is Credential => c != null)
	);

	// Get the network's default credential names for display
	let networkCredentialNames = $derived(() => {
		if (!network?.credential_ids?.length) return 'None';
		const names = network.credential_ids
			.map((id: string) => allCredentials.find((c) => c.id === id)?.name)
			.filter(Boolean);
		return names.length > 0 ? names.join(', ') : 'None';
	});

	function getAssignmentForIndex(index: number) {
		return (formData.credential_assignments ?? [])[index] ?? null;
	}

	function isAllInterfaces(index: number): boolean {
		const assignment = getAssignmentForIndex(index);
		return assignment ? assignment.interface_ids === null : true;
	}

	function toggleAllInterfaces(index: number) {
		const assignments = [...(formData.credential_assignments ?? [])];
		if (!assignments[index]) return;
		if (isAllInterfaces(index)) {
			assignments[index] = {
				...assignments[index],
				interface_ids: formData.interfaces.map((i) => i.id)
			};
		} else {
			assignments[index] = {
				...assignments[index],
				interface_ids: null
			};
		}
		formData.credential_assignments = assignments;
	}

	function toggleInterface(index: number, interfaceId: string) {
		if (isAllInterfaces(index)) return;
		const assignments = [...(formData.credential_assignments ?? [])];
		const current = assignments[index].interface_ids ?? [];
		if (current.includes(interfaceId)) {
			assignments[index] = {
				...assignments[index],
				interface_ids: current.filter((id) => id !== interfaceId)
			};
		} else {
			assignments[index] = {
				...assignments[index],
				interface_ids: [...current, interfaceId]
			};
		}
		formData.credential_assignments = assignments;
	}

	function isInterfaceChecked(index: number, interfaceId: string): boolean {
		const assignment = getAssignmentForIndex(index);
		if (!assignment) return false;
		if (assignment.interface_ids === null) return true;
		return assignment.interface_ids.includes(interfaceId);
	}
</script>

<ListConfigEditor items={selectedCredentials}>
	<svelte:fragment slot="list" let:items let:onEdit let:highlightedIndex>
		<div class="space-y-4">
			<p class="text-muted text-xs">
				Network default: {networkCredentialNames()}. Select credentials below to override for this
				host.
			</p>
			<ListManager
				label="Credential Override"
				helpText={isNonOwnerInDemo
					? 'Credential settings are read-only in demo mode.'
					: 'Select credentials to override the network defaults for this host.'}
				placeholder="Select a credential to add"
				emptyMessage="No credential overrides — using network defaults"
				allowReorder={false}
				options={allCredentials}
				{items}
				itemClickAction="edit"
				optionDisplayComponent={CredentialDisplay}
				itemDisplayComponent={CredentialDisplay}
				{onEdit}
				{highlightedIndex}
				onAdd={(id) => {
					const current = formData.credential_assignments ?? [];
					if (!current.some((a) => a.credential_id === id)) {
						formData.credential_assignments = [
							...current,
							{ credential_id: id, interface_ids: null }
						];
					}
				}}
				onRemove={(index) => {
					const current = formData.credential_assignments ?? [];
					formData.credential_assignments = current.filter((_, i) => i !== index);
				}}
			/>
		</div>
	</svelte:fragment>

	<svelte:fragment slot="config" let:selectedItem let:selectedIndex>
		{#if selectedItem && formData.interfaces.length > 0}
			<div class="space-y-4">
				<ConfigHeader
					title={selectedItem.name}
					subtitle="Configure which interfaces this credential applies to"
				/>
				<label class="flex items-center gap-2">
					<input
						type="checkbox"
						checked={isAllInterfaces(selectedIndex)}
						onchange={() => toggleAllInterfaces(selectedIndex)}
						class="checkbox"
					/>
					<span class="text-primary text-sm">All interfaces</span>
				</label>
				{#if !isAllInterfaces(selectedIndex)}
					<div class="space-y-1 pl-2">
						{#each formData.interfaces as iface (iface.id)}
							<label class="flex items-center gap-2">
								<input
									type="checkbox"
									checked={isInterfaceChecked(selectedIndex, iface.id)}
									onchange={() => toggleInterface(selectedIndex, iface.id)}
									class="checkbox"
								/>
								<span class="text-primary text-sm">
									{iface.ip_address}
									{#if iface.name}
										<span class="text-muted">({iface.name})</span>
									{/if}
								</span>
							</label>
						{/each}
					</div>
				{/if}
			</div>
		{:else if selectedItem}
			<EntityConfigEmpty
				title={selectedItem.name}
				subtitle="Add interfaces to this host to configure per-interface credential scope."
			/>
		{:else}
			<EntityConfigEmpty
				title="No Credential Selected"
				subtitle="Select a credential from the list to configure its interface scope."
			/>
		{/if}
	</svelte:fragment>
</ListConfigEditor>

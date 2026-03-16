<script lang="ts">
	import type { HostFormData } from '$lib/features/hosts/types/base';
	import type { Network } from '$lib/features/networks/types';
	import type { Credential } from '$lib/features/credentials/types/base';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
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

	// Resolve credential IDs to full objects
	let selectedCredentials = $derived(
		(formData.credential_ids ?? [])
			.map((id) => allCredentials.find((c) => c.id === id))
			.filter((c): c is Credential => c != null)
	);

	// Get the network's default credential names for display
	let networkCredentialNames = $derived(() => {
		if (!network?.credential_ids?.length) return 'None';
		const names = network.credential_ids
			.map((id) => allCredentials.find((c) => c.id === id)?.name)
			.filter(Boolean);
		return names.length > 0 ? names.join(', ') : 'None';
	});
</script>

<div class="space-y-6 p-6">
	<p class="text-muted text-xs">
		Network default: {networkCredentialNames()}. Select credentials below to override for this host.
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
		items={selectedCredentials}
		optionDisplayComponent={CredentialDisplay}
		itemDisplayComponent={CredentialDisplay}
		onAdd={(id) => {
			const current = formData.credential_ids ?? [];
			if (!current.includes(id)) {
				formData.credential_ids = [...current, id];
			}
		}}
		onRemove={(index) => {
			const current = formData.credential_ids ?? [];
			formData.credential_ids = current.filter((_, i) => i !== index);
		}}
	/>
</div>

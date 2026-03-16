<script lang="ts">
	import type { HostFormData } from '$lib/features/hosts/types/base';
	import type { Network } from '$lib/features/networks/types';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import InfoRow from '$lib/shared/components/data/InfoRow.svelte';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import { getCredentialTypeId } from '$lib/features/credentials/types/base';
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import {
		common_contact,
		common_location,
		hosts_snmp_chassisId,
		hosts_snmp_managementUrl,
		hosts_snmp_sysDescr,
		hosts_snmp_sysObjectId,
		hosts_snmp_systemInfo,
		hosts_snmp_systemInfoPending
	} from '$lib/paraglide/messages';

	interface Props {
		formData: HostFormData;
		form: {
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			Field: any;
			store: { subscribe: (fn: () => void) => () => void };
			state: { values: Record<string, unknown> };
		};
		isEditing: boolean;
		network?: Network | null;
	}

	let { formData = $bindable(), form, isEditing, network = null }: Props = $props();

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

	// Get the network's default credential names for display
	let networkCredentialNames = $derived(() => {
		if (!network?.credential_ids?.length) return 'None';
		const names = network.credential_ids
			.map((id) => allCredentials.find((c) => c.id === id)?.name)
			.filter(Boolean);
		return names.length > 0 ? names.join(', ') : 'None';
	});

	function toggleCredential(credentialId: string) {
		const current = formData.credential_ids ?? [];
		if (current.includes(credentialId)) {
			formData.credential_ids = current.filter((id) => id !== credentialId);
		} else {
			formData.credential_ids = [...current, credentialId];
		}
	}
</script>

<div class="space-y-6 p-6">
	<!-- Credential Selection Section -->
	<div class="space-y-4">
		<!-- svelte-ignore a11y_label_has_associated_control -->
		<label class="text-secondary block text-sm font-medium"> Credential Override </label>
		<p class="text-muted text-xs">
			Network default: {networkCredentialNames()}. Select credentials below to override for this
			host.
		</p>

		{#if allCredentials.length === 0}
			<p class="text-muted text-sm">
				No credentials available. Create credentials in the Credentials tab.
			</p>
		{:else}
			<div class="border-secondary max-h-48 space-y-1 overflow-y-auto rounded-md border p-2">
				{#each allCredentials as cred (cred.id)}
					{@const typeId = getCredentialTypeId(cred)}
					<label
						class="flex cursor-pointer items-center gap-3 rounded px-2 py-1.5 hover:bg-white/5"
						class:opacity-50={isNonOwnerInDemo}
					>
						<input
							type="checkbox"
							checked={(formData.credential_ids ?? []).includes(cred.id)}
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
	</div>

	<!-- SNMP System Information (read-only, only shown when editing) -->
	{#if isEditing}
		<InfoCard title={hosts_snmp_systemInfo()}>
			<InfoRow label={hosts_snmp_sysDescr()}>{formData.sys_descr || '-'}</InfoRow>
			<InfoRow label={hosts_snmp_sysObjectId()} mono>{formData.sys_object_id || '-'}</InfoRow>
			<InfoRow label={common_location()}>{formData.sys_location || '-'}</InfoRow>
			<InfoRow label={common_contact()}>{formData.sys_contact || '-'}</InfoRow>
			<InfoRow label={hosts_snmp_chassisId()} mono>{formData.chassis_id || '-'}</InfoRow>
			<InfoRow label={hosts_snmp_managementUrl()}>
				{#if formData.management_url}
					<!-- eslint-disable svelte/no-navigation-without-resolve -->
					<a
						href={formData.management_url}
						target="_blank"
						rel="external noopener noreferrer"
						class="break-all text-blue-400 hover:text-blue-300"
					>
						{formData.management_url}
					</a>
					<!-- eslint-enable svelte/no-navigation-without-resolve -->
				{:else}
					-
				{/if}
			</InfoRow>
		</InfoCard>
	{/if}

	{#if !isEditing}
		<div class="bg-tertiary/30 rounded-lg p-4">
			<p class="text-muted text-sm">
				{hosts_snmp_systemInfoPending()}
			</p>
		</div>
	{/if}
</div>

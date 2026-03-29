<script lang="ts">
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import { SubnetDisplay } from '$lib/shared/components/forms/selection/display/SubnetDisplay.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import type { Discovery } from '../../types/base';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import { subnetTypes } from '$lib/shared/stores/metadata';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import {
		discovery_allSubnetsScanned,
		discovery_daemonHostMissing,
		discovery_daemonHostMissingHelp,
		discovery_minSubnetPrefix,
		discovery_minSubnetPrefixHelp,
		discovery_nonInterfacedSubnet,
		discovery_nonInterfacedSubnetWarning,
		discovery_selectSubnet,
		discovery_subnetsExcludedByPrefix,
		discovery_subnetsExcludedByPrefixWarning,
		discovery_targetSubnets,
		discovery_targetSubnetsHelp
	} from '$lib/paraglide/messages';

	interface Props {
		formData: Discovery;
		daemonHostId: string | null;
		daemon: Daemon;
	}

	let { formData = $bindable(), daemonHostId, daemon }: Props = $props();

	const subnetsQuery = useSubnetsQuery();

	let subnetsData = $derived(subnetsQuery.data ?? []);

	let availableSubnets = $derived(
		subnetsData.filter(
			(s) =>
				(formData.discovery_type.type === 'Network' ||
					formData.discovery_type.type === 'Unified') &&
				s.network_id == formData.network_id &&
				!formData.discovery_type.subnet_ids?.includes(s.id) &&
				subnetTypes.getMetadata(s.subnet_type).network_scan_discovery_eligible
		)
	);

	let selectedSubnets = $derived(
		(formData.discovery_type.type === 'Network' || formData.discovery_type.type === 'Unified') &&
			formData.discovery_type.subnet_ids
			? formData.discovery_type.subnet_ids
					.map((id) => subnetsData.find((s) => s.id === id))
					.filter(Boolean)
			: []
	);

	let nonInterfacedSubnets = $derived(
		(formData.discovery_type.type == 'Network' || formData.discovery_type.type == 'Unified') &&
			formData.discovery_type.subnet_ids &&
			formData.discovery_type.subnet_ids.length > 0
			? formData.discovery_type.subnet_ids
					.filter((s) => !daemon.capabilities.interfaced_subnet_ids.includes(s))
					.map((s) => subnetsData.find((subnet) => subnet.id == s))
					.filter((s) => s != undefined)
					.map((s) => (s.name !== s.cidr ? s.name + ` (${s.cidr})` : s.cidr))
			: []
	);

	// Parse CIDR prefix length from a string like "192.168.1.0/24" → 24
	function getCidrPrefix(cidr: string): number | null {
		const parts = cidr.split('/');
		if (parts.length !== 2) return null;
		const prefix = parseInt(parts[1], 10);
		return isNaN(prefix) ? null : prefix;
	}

	let minSubnetPrefix = $derived(
		formData.discovery_type.type === 'Unified'
			? (formData.discovery_type.scan_settings?.min_subnet_prefix ?? 16)
			: 16
	);

	// Find selected non-interfaced subnets that would be excluded by min prefix
	let prefixExcludedSubnets = $derived(
		(formData.discovery_type.type == 'Network' || formData.discovery_type.type == 'Unified') &&
			formData.discovery_type.subnet_ids &&
			formData.discovery_type.subnet_ids.length > 0
			? formData.discovery_type.subnet_ids
					.filter((s) => !daemon.capabilities.interfaced_subnet_ids.includes(s))
					.map((s) => subnetsData.find((subnet) => subnet.id == s))
					.filter((s) => s != undefined)
					.filter((s) => {
						const prefix = getCidrPrefix(s.cidr);
						return prefix !== null && prefix < minSubnetPrefix;
					})
					.map((s) => (s.name !== s.cidr ? s.name + ` (${s.cidr})` : s.cidr))
			: []
	);

	function handleAddSubnet(subnetId: string) {
		if (formData.discovery_type.type === 'Network' || formData.discovery_type.type === 'Unified') {
			const currentIds = formData.discovery_type.subnet_ids || [];
			formData.discovery_type = {
				...formData.discovery_type,
				subnet_ids: [...currentIds, subnetId]
			};
		}
	}

	function handleRemoveSubnet(index: number) {
		if (
			(formData.discovery_type.type === 'Network' || formData.discovery_type.type === 'Unified') &&
			formData.discovery_type.subnet_ids
		) {
			formData.discovery_type = {
				...formData.discovery_type,
				subnet_ids: formData.discovery_type.subnet_ids.filter((_, i) => i !== index)
			};
		}
	}

	function updateMinSubnetPrefix(value: number) {
		if (formData.discovery_type.type !== 'Unified') return;
		const current = formData.discovery_type.scan_settings ?? {};
		if (isNaN(value)) {
			formData.discovery_type = {
				...formData.discovery_type,
				scan_settings: { ...current, min_subnet_prefix: null }
			};
		} else {
			formData.discovery_type = {
				...formData.discovery_type,
				scan_settings: { ...current, min_subnet_prefix: value }
			};
		}
	}
</script>

<div class="space-y-4">
	{#if daemonHostId == null}
		<InlineWarning title={discovery_daemonHostMissing()} body={discovery_daemonHostMissingHelp()} />
	{/if}

	{#if formData.discovery_type.type === 'Network' || formData.discovery_type.type === 'Unified'}
		<div class="card">
			<ListManager
				label={discovery_targetSubnets()}
				helpText={discovery_targetSubnetsHelp()}
				placeholder={discovery_selectSubnet()}
				emptyMessage={discovery_allSubnetsScanned()}
				allowReorder={false}
				allowItemEdit={() => false}
				showSearch={true}
				options={availableSubnets}
				items={selectedSubnets}
				optionDisplayComponent={SubnetDisplay}
				itemDisplayComponent={SubnetDisplay}
				onAdd={handleAddSubnet}
				onRemove={handleRemoveSubnet}
			/>
		</div>
		{#if nonInterfacedSubnets.length > 0}
			<InlineWarning
				title={discovery_nonInterfacedSubnet()}
				body={discovery_nonInterfacedSubnetWarning({
					subnets: nonInterfacedSubnets.join('\n')
				})}
			/>
		{/if}

		{#if formData.discovery_type.type === 'Unified'}
			<div class="card space-y-2">
				<label for="min_subnet_prefix" class="text-secondary block text-sm font-medium">
					{discovery_minSubnetPrefix()}
				</label>
				<input
					id="min_subnet_prefix"
					type="number"
					value={minSubnetPrefix}
					oninput={(e) => updateMinSubnetPrefix(Number(e.currentTarget.value))}
					placeholder="16"
					min="2"
					max="32"
					class="input-field"
				/>
				<p class="text-tertiary text-xs">{discovery_minSubnetPrefixHelp()}</p>
			</div>
		{/if}

		{#if prefixExcludedSubnets.length > 0}
			<InlineWarning
				title={discovery_subnetsExcludedByPrefix()}
				body={discovery_subnetsExcludedByPrefixWarning({
					minPrefix: String(minSubnetPrefix),
					subnets: prefixExcludedSubnets.join('\n')
				})}
			/>
		{/if}
	{/if}
</div>

<script lang="ts">
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import { SubnetDisplay } from '$lib/shared/components/forms/selection/display/SubnetDisplay.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import type { Discovery } from '../../types/base';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import { subnetTypes } from '$lib/shared/stores/metadata';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import {
		common_ipAddress,
		discovery_allSubnetsScanned,
		discovery_bestService,
		discovery_daemonHostMissing,
		discovery_daemonHostMissingHelp,
		discovery_hostNameFallback,
		discovery_hostNameFallbackHelp,
		discovery_nonInterfacedSubnet,
		discovery_nonInterfacedSubnetWarning,
		discovery_scanLocalDockerSocket,
		discovery_scanLocalDockerSocketHelp,
		discovery_selectSubnet,
		discovery_targetSubnets,
		discovery_targetSubnetsHelp
	} from '$lib/paraglide/messages';

	interface Props {
		/* eslint-disable @typescript-eslint/no-explicit-any */
		form: any;
		/* eslint-enable @typescript-eslint/no-explicit-any */
		formData: Discovery;
		readOnly?: boolean;
		daemonHostId: string | null;
		daemon: Daemon;
	}

	let { form, formData = $bindable(), readOnly = false, daemonHostId, daemon }: Props = $props();

	const subnetsQuery = useSubnetsQuery();

	let subnetsData = $derived(subnetsQuery.data ?? []);

	let hostNameFallbackOptions = $derived([
		{ value: 'Ip', label: common_ipAddress() },
		{ value: 'BestService', label: discovery_bestService() }
	]);

	function handleHostNameFallbackChange(value: string) {
		if (
			formData.discovery_type.type == 'Docker' ||
			formData.discovery_type.type == 'Network' ||
			formData.discovery_type.type == 'Unified'
		) {
			if (formData.discovery_type.host_naming_fallback !== value) {
				formData.discovery_type = {
					...formData.discovery_type,
					host_naming_fallback: value as 'BestService' | 'Ip'
				};
			}
		}
	}

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
					.map((s) => s.name + ` (${s.cidr})`)
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
</script>

<div class="space-y-4">
	{#if daemonHostId == null}
		<InlineWarning title={discovery_daemonHostMissing()} body={discovery_daemonHostMissingHelp()} />
	{/if}

	{#if formData.discovery_type.type == 'Docker' || formData.discovery_type.type == 'Network' || formData.discovery_type.type == 'Unified'}
		<div class="card">
			<form.Field
				name="host_naming_fallback"
				listeners={{
					onChange: ({ value }: { value: string }) => handleHostNameFallbackChange(value)
				}}
			>
				{#snippet children(field: AnyFieldApi)}
					<SelectInput
						label={discovery_hostNameFallback()}
						id="host_name_fallback"
						options={hostNameFallbackOptions}
						{field}
						disabled={readOnly}
						helpText={discovery_hostNameFallbackHelp()}
					/>
				{/snippet}
			</form.Field>
		</div>
	{/if}

	{#if formData.discovery_type.type === 'Unified'}
		<div class="card">
			<div class="flex flex-col gap-2">
				<label
					for="scan_local_docker_socket"
					class="text-secondary flex cursor-pointer items-center gap-2 text-sm font-medium"
				>
					<input
						type="checkbox"
						id="scan_local_docker_socket"
						checked={formData.discovery_type.scan_local_docker_socket ?? false}
						disabled={readOnly}
						onchange={(e) => {
							if (formData.discovery_type.type === 'Unified') {
								formData.discovery_type = {
									...formData.discovery_type,
									scan_local_docker_socket: e.currentTarget.checked
								};
							}
						}}
						class="checkbox-card h-4 w-4 focus:ring-1 focus:ring-blue-500 disabled:cursor-not-allowed disabled:opacity-50"
					/>
					<div>{discovery_scanLocalDockerSocket()}</div>
				</label>
				<p class="text-tertiary text-xs">{discovery_scanLocalDockerSocketHelp()}</p>
			</div>
		</div>
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
	{/if}
</div>

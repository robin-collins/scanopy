<script lang="ts">
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import { required, max } from '$lib/shared/components/forms/validators';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import { DaemonDisplay } from '$lib/shared/components/forms/selection/display/DaemonDisplay.svelte';
	import {
		SimpleOptionDisplay,
		type SimpleOption
	} from '$lib/shared/components/forms/selection/display/SimpleOptionDisplay';
	import type { DockerDiscovery, NetworkDiscovery, SelfReportDiscovery } from '../../types/api';
	import type { Discovery } from '../../types/base';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import type { Host } from '$lib/features/hosts/types/base';
	import type { Subnet } from '$lib/features/subnets/types/base';
	import { discoveryTypes } from '$lib/shared/stores/metadata';
	import { openModal } from '$lib/shared/stores/modal-registry';
	import { ArrowUpCircle } from 'lucide-svelte';
	import {
		common_daemon,
		discovery_adHoc,
		discovery_adHocDescription,
		discovery_daemonHelp,
		discovery_daemonSelect,
		discovery_discoveryType,
		discovery_dockerScan,
		discovery_name,
		discovery_namePlaceholder,
		discovery_networkScan,
		discovery_runType,
		discovery_scheduled,
		discovery_scheduledDescription,
		discovery_selfReport
	} from '$lib/paraglide/messages';

	interface Props {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any; state: { values: { name: string } } };
		formData: Discovery;
		daemons?: Daemon[];
		hosts?: Host[];
		subnets?: Subnet[];
		readOnly?: boolean;
		hasScheduledDiscovery?: boolean;
		daemonHostId?: string | null;
		daemon?: Daemon | null;
	}

	let {
		form,
		formData = $bindable(),
		daemons = [],
		hosts = [],
		subnets = [],
		readOnly = false,
		hasScheduledDiscovery = true,
		daemonHostId = null,
		daemon = null
	}: Props = $props();

	let discoveryTypeOptions = $derived([
		{ value: 'Network', label: discovery_networkScan(), disabled: false },
		{
			value: 'Docker',
			label: discovery_dockerScan(),
			disabled: daemonHostId == null || !daemon?.capabilities.has_docker_socket
		},
		{ value: 'SelfReport', label: discovery_selfReport(), disabled: daemonHostId == null }
	]);

	function handleDiscoveryTypeChange(value: string) {
		if (value === 'Network' && formData.discovery_type.type !== 'Network') {
			formData.discovery_type = {
				type: 'Network',
				subnet_ids: daemon?.capabilities.interfaced_subnet_ids ?? [],
				host_naming_fallback: 'BestService'
			} as NetworkDiscovery;
		} else if (value === 'Docker' && formData.discovery_type.type !== 'Docker') {
			formData.discovery_type = {
				type: 'Docker',
				host_id: daemonHostId,
				host_naming_fallback: 'BestService'
			} as DockerDiscovery;
		} else if (value === 'SelfReport' && formData.discovery_type.type !== 'SelfReport') {
			formData.discovery_type = {
				type: 'SelfReport',
				host_id: daemonHostId
			} as SelfReportDiscovery;
		}
	}

	let runTypeOptions: SimpleOption[] = $derived([
		{ value: 'AdHoc', label: discovery_adHoc() },
		{
			value: 'Scheduled',
			label: discovery_scheduled(),
			disabled: !hasScheduledDiscovery,
			tags: !hasScheduledDiscovery
				? [
						{
							label: 'Upgrade',
							color: 'Yellow',
							icon: ArrowUpCircle
						}
					]
				: []
		}
	]);

	function handleRunTypeChange(value: string) {
		if (value === 'AdHoc' && formData.run_type.type !== 'AdHoc') {
			formData.run_type = {
				type: 'AdHoc',
				last_run: null
			};
		} else if (value === 'Scheduled' && formData.run_type.type !== 'Scheduled') {
			formData.run_type = {
				type: 'Scheduled',
				cron_schedule: '0 0 0 * * 0',
				last_run: null,
				enabled: true,
				timezone: Intl.DateTimeFormat().resolvedOptions().timeZone
			};
		}
	}
</script>

<div class="space-y-4">
	<form.Field
		name="name"
		validators={{
			onBlur: ({ value }: { value: string }) => required(value) || max(100)(value)
		}}
	>
		{#snippet children(field: AnyFieldApi)}
			<TextInput
				label={discovery_name()}
				id="name"
				placeholder={discovery_namePlaceholder()}
				required={true}
				{field}
				disabled={readOnly}
			/>
		{/snippet}
	</form.Field>

	<!-- Daemon Selection -->
	<div class="space-y-2">
		<RichSelect
			label={common_daemon()}
			required={true}
			placeholder={discovery_daemonSelect()}
			disabled={readOnly}
			selectedValue={formData.daemon_id}
			options={daemons}
			displayComponent={DaemonDisplay}
			getOptionContext={() => ({ hosts, subnets })}
			onSelect={(value) => {
				const selectedDaemon = daemons.find((d) => d.id === value);
				if (selectedDaemon) {
					formData = { ...formData, daemon_id: value, network_id: selectedDaemon.network_id };
				}
			}}
		/>
		<p class="text-tertiary text-xs">{discovery_daemonHelp()}</p>
	</div>

	<!-- Run Type Selection -->
	<form.Field
		name="run_type_type"
		listeners={{
			onChange: ({ value }: { value: string }) => handleRunTypeChange(value)
		}}
	>
		{#snippet children(field: AnyFieldApi)}
			<RichSelect
				label={discovery_runType()}
				selectedValue={field.state.value}
				options={runTypeOptions}
				onSelect={(value) => field.handleChange(value)}
				onDisabledClick={() => openModal('billing-plan')}
				displayComponent={SimpleOptionDisplay}
				disabled={readOnly}
			/>
			<p class="text-tertiary mt-1 text-xs">
				{field.state.value === 'AdHoc'
					? discovery_adHocDescription()
					: discovery_scheduledDescription()}
			</p>
		{/snippet}
	</form.Field>

	<!-- Discovery Type Selection -->
	{#if daemon}
		<form.Field
			name="discovery_type_type"
			listeners={{
				onChange: ({ value }: { value: string }) => handleDiscoveryTypeChange(value)
			}}
		>
			{#snippet children(field: AnyFieldApi)}
				<SelectInput
					label={discovery_discoveryType()}
					id="discovery_type"
					options={discoveryTypeOptions}
					{field}
					disabled={readOnly}
				/>
				<p class="text-tertiary mt-1 text-xs">
					{discoveryTypes.getDescription(field.state.value)}
				</p>
			{/snippet}
		</form.Field>
	{/if}
</div>

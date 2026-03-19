<script lang="ts">
	import ProgressTrack from '$lib/shared/components/data/ProgressTrack.svelte';
	import InlineSuccess from '$lib/shared/components/feedback/InlineSuccess.svelte';
	import InlineDanger from '$lib/shared/components/feedback/InlineDanger.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import type { DiscoveryUpdatePayload } from '../../types/api';
	import { formatDuration, formatTimestamp } from '$lib/shared/utils/formatting';
	import { useSubnetsQuery, getSubnetById } from '$lib/features/subnets/queries';
	import scanSettingsFields from '$lib/data/scan-settings.json';
	import {
		discovery_scanDetails,
		discovery_dockerScanning,
		discovery_hostNamingFallback,
		discovery_scanSettings,
		discovery_defaultSettings,
		discovery_bestService,
		common_ipAddress,
		common_enabled,
		common_disabled
	} from '$lib/paraglide/messages';

	interface Props {
		payload: DiscoveryUpdatePayload;
	}

	let { payload }: Props = $props();

	// TanStack Query for subnets
	const subnetsQuery = useSubnetsQuery();
	let subnetsData = $derived(subnetsQuery.data ?? []);

	let duration = $derived(
		payload.started_at && payload.finished_at
			? formatDuration(payload.started_at, payload.finished_at)
			: null
	);

	// Helper to get subnet name by ID
	function getSubnetName(subnetId: string): string {
		const subnet = getSubnetById(subnetsData, subnetId);
		return subnet?.name || 'Unknown Subnet';
	}

	// Scan settings field metadata for label lookup
	interface FieldDef {
		id: string;
		label: string;
		default_value: string;
		field_type: string;
	}

	const fields = scanSettingsFields as FieldDef[];

	// Get non-default scan settings as label/value pairs
	let nonDefaultSettings = $derived.by(() => {
		if (payload.discovery_type.type !== 'Unified' || !payload.discovery_type.scan_settings) {
			return [];
		}
		const settings = payload.discovery_type.scan_settings;
		const result: { label: string; value: string }[] = [];
		for (const field of fields) {
			const val = settings[field.id as keyof typeof settings];
			if (val !== undefined && val !== null && String(val) !== field.default_value) {
				result.push({
					label: field.label,
					value: field.field_type === 'boolean' ? (val ? 'Yes' : 'No') : String(val)
				});
			}
		}
		return result;
	});

	let hostNamingLabel = $derived(
		payload.discovery_type.type === 'Unified'
			? payload.discovery_type.host_naming_fallback === 'Ip'
				? common_ipAddress()
				: discovery_bestService()
			: ''
	);
</script>

<div class="space-y-4 border-t pt-6" style="border-color: var(--color-border)">
	<h3 class="text-primary text-lg font-medium">Discovery Run Summary</h3>

	<!-- Status Banner -->
	{#if payload.phase === 'Complete'}
		<InlineSuccess title={payload.phase} />
	{:else if payload.phase === 'Failed'}
		<InlineDanger title={payload.phase} body={payload.error ?? null} />
	{:else if payload.phase === 'Cancelled'}
		<InlineWarning title={payload.phase} />
	{:else}
		<InlineInfo title={payload.phase} />
	{/if}

	<!-- Details Grid -->
	<div class="grid grid-cols-2 gap-4">
		<!-- Processed -->
		{#if payload.progress !== undefined}
			<div class="card p-4">
				<div class="text-tertiary mb-1 text-xs font-medium uppercase tracking-wide">Progress</div>
				<div class="flex items-center gap-2">
					<div class="text-secondary text-sm">
						{payload.progress}%
					</div>
					<ProgressTrack progress={payload.progress} class="flex-1" />
				</div>
			</div>
		{/if}

		<!-- Duration -->
		{#if duration}
			<div class="card p-4">
				<div class="text-tertiary mb-1 text-xs font-medium uppercase tracking-wide">Duration</div>
				<div class="text-secondary text-sm">{duration}</div>
			</div>
		{/if}

		<!-- Start Time -->
		{#if payload.started_at}
			<div class="card p-4">
				<div class="text-tertiary mb-1 text-xs font-medium uppercase tracking-wide">Started</div>
				<div class="text-secondary text-sm">{formatTimestamp(payload.started_at)}</div>
			</div>
		{/if}

		<!-- End Time -->
		{#if payload.finished_at}
			<div class="card p-4">
				<div class="text-tertiary mb-1 text-xs font-medium uppercase tracking-wide">Finished</div>
				<div class="text-secondary text-sm">{formatTimestamp(payload.finished_at)}</div>
			</div>
		{/if}
	</div>

	<!-- Type-specific Details -->
	{#if payload.discovery_type.type === 'Network' || payload.discovery_type.type === 'Unified'}
		<div class="card p-4">
			<div class="text-tertiary mb-2 text-xs font-medium uppercase tracking-wide">
				{discovery_scanDetails()}
			</div>
			<div class="text-secondary text-sm">
				{#if payload.discovery_type.subnet_ids === null}
					Scanned all subnets that daemon had an interface with at time of scan
				{:else}
					Scanned {payload.discovery_type.subnet_ids.map((s) => getSubnetName(s)).join(', ')}
				{/if}
			</div>
		</div>

		{#if payload.discovery_type.type === 'Unified'}
			<div class="grid grid-cols-2 gap-4">
				<!-- Docker Scanning -->
				<div class="card p-4">
					<div class="text-tertiary mb-1 text-xs font-medium uppercase tracking-wide">
						{discovery_dockerScanning()}
					</div>
					<div class="text-secondary text-sm">
						{payload.discovery_type.scan_local_docker_socket ? common_enabled() : common_disabled()}
					</div>
				</div>

				<!-- Host Naming Fallback -->
				<div class="card p-4">
					<div class="text-tertiary mb-1 text-xs font-medium uppercase tracking-wide">
						{discovery_hostNamingFallback()}
					</div>
					<div class="text-secondary text-sm">{hostNamingLabel}</div>
				</div>
			</div>

			<!-- Scan Settings -->
			<div class="card p-4">
				<div class="text-tertiary mb-2 text-xs font-medium uppercase tracking-wide">
					{discovery_scanSettings()}
				</div>
				{#if nonDefaultSettings.length === 0}
					<div class="text-secondary text-sm">{discovery_defaultSettings()}</div>
				{:else}
					<div class="flex flex-wrap gap-x-4 gap-y-1">
						{#each nonDefaultSettings as setting (setting.label)}
							<div class="text-secondary text-sm">
								<span class="text-tertiary">{setting.label}:</span>
								{setting.value}
							</div>
						{/each}
					</div>
				{/if}
			</div>
		{/if}
	{:else if payload.discovery_type.type === 'Docker'}
		<div class="card p-4">
			<div class="text-tertiary mb-2 text-xs font-medium uppercase tracking-wide">
				Docker Scan Details
			</div>
			<div class="text-secondary font-mono text-sm">
				Host ID: {payload.discovery_type.host_id}
			</div>
		</div>
	{:else if payload.discovery_type.type === 'SelfReport'}
		<div class="card p-4">
			<div class="text-tertiary mb-2 text-xs font-medium uppercase tracking-wide">
				Self Report Details
			</div>
			<div class="text-secondary font-mono text-sm">
				Host ID: {payload.discovery_type.host_id}
			</div>
		</div>
	{/if}
</div>

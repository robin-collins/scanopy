<script lang="ts">
	import scanSettingsFields from '$lib/data/scan-settings.json';
	import CollapsibleCard from '$lib/shared/components/data/CollapsibleCard.svelte';
	import type { Discovery } from '../../types/base';
	import { serviceDefinitions } from '$lib/shared/stores/metadata';
	import {
		discovery_forceFullScan,
		discovery_forceFullScanHelp,
		discovery_scanModeIntervalExplainer,
		discovery_scanSettingsHelp
	} from '$lib/paraglide/messages';

	interface Props {
		formData: Discovery;
		readOnly?: boolean;
	}

	let { formData = $bindable(), readOnly = false }: Props = $props();

	type FieldDef = {
		id: string;
		label: string;
		field_type: string;
		placeholder?: string;
		help_text?: string;
		default_value?: string;
		optional?: boolean;
		category?: string;
	};

	const fields = scanSettingsFields as FieldDef[];

	// Exclude interfaces (moved to Targets tab) and group by category
	const speedFields = fields.filter((f) => f.id !== 'interfaces');

	// Group fields by category, falling back to a single group if no categories defined
	const categories = [...new Set(speedFields.map((f) => f.category).filter(Boolean))] as string[];
	const fieldsByCategory =
		categories.length > 0
			? categories.map((cat) => ({
					name: cat,
					fields: speedFields.filter((f) => f.category === cat)
				}))
			: [{ name: 'Scan Settings', fields: speedFields }];

	let rawSocketServiceNames = $derived(
		(serviceDefinitions.getItems() ?? [])
			.filter((s) => s.metadata?.has_raw_socket_endpoint)
			.map((s) => s.name)
			.join(', ')
	);

	function getHelpText(field: FieldDef): string {
		if (field.id === 'probe_raw_socket_ports' && rawSocketServiceNames) {
			return `${field.help_text} Required to detect: ${rawSocketServiceNames}`;
		}
		return field.help_text ?? '';
	}

	// scan_settings lives inside discovery_type for Unified variant
	function getScanSettings() {
		if (formData.discovery_type.type === 'Unified') {
			return formData.discovery_type.scan_settings ?? {};
		}
		return {};
	}

	// Use explicit $derived with named property access so Svelte 5 can track reactivity.
	// Numeric fields: null from API → empty string → placeholder shows.
	let scanValues = $derived({
		scan_rate_pps: getScanSettings().scan_rate_pps ?? '',
		arp_rate_pps: getScanSettings().arp_rate_pps ?? '',
		arp_retries: getScanSettings().arp_retries ?? '',
		port_scan_batch_size: getScanSettings().port_scan_batch_size ?? '',
		probe_raw_socket_ports: getScanSettings().probe_raw_socket_ports ?? false,
		use_npcap_arp: getScanSettings().use_npcap_arp ?? false,
		full_scan_interval: getScanSettings().full_scan_interval ?? ''
	});

	function getScanValue(id: string): string | boolean | number {
		return (scanValues as Record<string, string | boolean | number>)[id] ?? '';
	}

	function updateScanSetting(id: string, value: string | boolean | number) {
		if (formData.discovery_type.type !== 'Unified') return;
		const current = formData.discovery_type.scan_settings ?? {};
		if (typeof value === 'number' && isNaN(value)) {
			formData.discovery_type = {
				...formData.discovery_type,
				scan_settings: { ...current, [id]: null }
			};
		} else {
			formData.discovery_type = {
				...formData.discovery_type,
				scan_settings: { ...current, [id]: value }
			};
		}
	}

	let isPortScanningCategory = (name: string) => name === 'Port Scanning';
</script>

<div class="space-y-4">
	<p class="text-tertiary text-sm">{discovery_scanSettingsHelp()}</p>

	{#each fieldsByCategory as category (category.name)}
		<CollapsibleCard title={category.name} expanded={true}>
			<div class="space-y-3">
				{@const numberFields = category.fields.filter((f) => f.field_type !== 'boolean')}
				{@const booleanFields = category.fields.filter((f) => f.field_type === 'boolean')}

				{#each numberFields as field (field.id)}
					<div class="space-y-2">
						<label for={`scan_${field.id}`} class="text-secondary block text-sm font-medium">
							{field.label}
						</label>
						<input
							id={`scan_${field.id}`}
							type="number"
							value={getScanValue(field.id)}
							oninput={(e) => updateScanSetting(field.id, Number(e.currentTarget.value))}
							placeholder={field.placeholder ?? ''}
							disabled={readOnly}
							class="input-field"
						/>
						{#if getHelpText(field)}
							<p class="text-tertiary text-xs">{getHelpText(field)}</p>
						{/if}
						{#if field.id === 'full_scan_interval'}
							<p class="text-tertiary text-xs italic">{discovery_scanModeIntervalExplainer()}</p>
						{/if}
					</div>
				{/each}

				{#if booleanFields.length > 0 || isPortScanningCategory(category.name)}
					<div class="grid grid-cols-2 gap-4 pt-2">
						{#each booleanFields as field (field.id)}
							<div class="flex flex-col gap-1">
								<label
									for={`scan_${field.id}`}
									class="text-secondary flex cursor-pointer items-center gap-2 text-sm font-medium"
								>
									<input
										type="checkbox"
										id={`scan_${field.id}`}
										checked={!!getScanValue(field.id)}
										disabled={readOnly}
										onchange={(e) => updateScanSetting(field.id, e.currentTarget.checked)}
										class="checkbox-card h-4 w-4 focus:ring-1 focus:ring-blue-500 disabled:cursor-not-allowed disabled:opacity-50"
									/>
									<div>{field.label}</div>
								</label>
								{#if getHelpText(field)}
									<p class="text-tertiary text-xs">{getHelpText(field)}</p>
								{/if}
							</div>
						{/each}

						{#if isPortScanningCategory(category.name) && formData.discovery_type.type === 'Unified'}
							<div class="flex flex-col gap-1">
								<label
									for="scan_force_full_scan"
									class="text-secondary flex cursor-pointer items-center gap-2 text-sm font-medium"
								>
									<input
										type="checkbox"
										id="scan_force_full_scan"
										checked={formData.force_full_scan ?? false}
										disabled={readOnly}
										onchange={(e) => {
											formData.force_full_scan = e.currentTarget.checked;
										}}
										class="checkbox-card h-4 w-4 focus:ring-1 focus:ring-blue-500 disabled:cursor-not-allowed disabled:opacity-50"
									/>
									<div>{discovery_forceFullScan()}</div>
								</label>
								<p class="text-tertiary text-xs">{discovery_forceFullScanHelp()}</p>
							</div>
						{/if}
					</div>
				{/if}
			</div>
		</CollapsibleCard>
	{/each}
</div>

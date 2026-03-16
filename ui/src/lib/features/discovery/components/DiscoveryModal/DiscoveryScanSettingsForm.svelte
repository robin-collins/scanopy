<script lang="ts">
	import scanSettingsFields from '$lib/data/scan-settings.json';
	import CollapsibleCard from '$lib/shared/components/data/CollapsibleCard.svelte';
	import type { Discovery } from '../../types/base';
	import { serviceDefinitions } from '$lib/shared/stores/metadata';
	import { discovery_scanSettingsHelp } from '$lib/paraglide/messages';

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
	const speedFields = fields.filter((f) => f.category !== 'Targets');

	// Group fields by category
	const categories = [...new Set(speedFields.map((f) => f.category).filter(Boolean))] as string[];
	const fieldsByCategory = categories.map((cat) => ({
		name: cat,
		fields: speedFields.filter((f) => f.category === cat)
	}));

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

	function getScanValue(id: string): string | boolean | number {
		if (!formData.scan_settings) return '';
		const val = (formData.scan_settings as Record<string, unknown>)[id];
		if (val === undefined || val === null) return '';
		return val as string | boolean | number;
	}

	function updateScanSetting(id: string, value: string | boolean | number) {
		if (!formData.scan_settings) return;
		formData.scan_settings = {
			...formData.scan_settings,
			[id]: value
		};
	}
</script>

<div class="space-y-4">
	<p class="text-tertiary text-sm">{discovery_scanSettingsHelp()}</p>

	{#each fieldsByCategory as category (category.name)}
		<CollapsibleCard title={category.name} expanded={true}>
			<div class="space-y-3">
				{#each category.fields as field (field.id)}
					{#if field.field_type === 'boolean'}
						<div class="flex flex-col gap-2">
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
					{:else}
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
						</div>
					{/if}
				{/each}
			</div>
		</CollapsibleCard>
	{/each}
</div>

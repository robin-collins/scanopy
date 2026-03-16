<script lang="ts">
	import scanSettingsFields from '$lib/data/scan-settings.json';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import Checkbox from '$lib/shared/components/forms/input/Checkbox.svelte';
	import type { Discovery } from '../../types/base';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import { serviceDefinitions } from '$lib/shared/stores/metadata';

	interface Props {
		/* eslint-disable @typescript-eslint/no-explicit-any */
		form: any;
		/* eslint-enable @typescript-eslint/no-explicit-any */
		formData: Discovery;
		readOnly?: boolean;
	}

	let { form, formData = $bindable(), readOnly = false }: Props = $props();

	type FieldDef = {
		id: string;
		label: string;
		field_type: string;
		placeholder?: string;
		help_text?: string;
		default_value?: string;
		optional?: boolean;
	};

	const fields = scanSettingsFields as FieldDef[];

	// Group fields by section
	const hostDiscoveryFields = fields.filter((f) =>
		['arp_rate_pps', 'arp_retries', 'use_npcap_arp'].includes(f.id)
	);
	const portScanningFields = fields.filter((f) =>
		['scan_rate_pps', 'port_scan_batch_size', 'probe_raw_socket_ports'].includes(f.id)
	);
	const advancedFields = fields.filter((f) => ['interfaces'].includes(f.id));

	let rawSocketServiceNames = $derived(
		(serviceDefinitions.getItems() ?? [])
			.filter((s) => s.metadata?.has_raw_socket_endpoint)
			.map((s) => s.name)
			.join(', ')
	);

	let expanded = $state(false);

	function updateScanSetting(id: string, value: string | boolean | number) {
		if (!formData.scan_settings) return;
		if (id === 'interfaces') {
			formData.scan_settings = {
				...formData.scan_settings,
				interfaces:
					typeof value === 'string'
						? value
								.split(',')
								.map((s) => s.trim())
								.filter((s) => s.length > 0)
						: []
			};
		} else {
			formData.scan_settings = {
				...formData.scan_settings,
				[id]: value
			};
		}
	}

	function getHelpText(field: FieldDef): string {
		if (field.id === 'probe_raw_socket_ports' && rawSocketServiceNames) {
			return `${field.help_text} Required to detect: ${rawSocketServiceNames}`;
		}
		return field.help_text ?? '';
	}
</script>

<div class="space-y-1">
	<button
		type="button"
		class="flex w-full items-center gap-2 text-left text-sm font-medium text-gray-700 dark:text-gray-300"
		onclick={() => (expanded = !expanded)}
	>
		<svg
			class="h-4 w-4 transition-transform {expanded ? 'rotate-90' : ''}"
			fill="none"
			viewBox="0 0 24 24"
			stroke="currentColor"
		>
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
		</svg>
		Scan Settings
		<span class="text-xs font-normal text-gray-500">(defaults are usually fine)</span>
	</button>

	{#if expanded}
		<div class="space-y-6 pl-6 pt-3">
			<!-- Host Discovery -->
			<div class="space-y-3">
				<h4 class="text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-gray-400">
					Host Discovery
				</h4>
				{#each hostDiscoveryFields as field (field.id)}
					{#if field.field_type === 'boolean'}
						<form.Field
							name={`scan_${field.id}`}
							listeners={{
								onChange: ({ value }: { value: boolean }) => updateScanSetting(field.id, value)
							}}
						>
							{#snippet children(formField: AnyFieldApi)}
								<Checkbox
									label={field.label}
									id={`scan_${field.id}`}
									field={formField}
									disabled={readOnly}
									helpText={getHelpText(field)}
								/>
							{/snippet}
						</form.Field>
					{:else}
						<form.Field
							name={`scan_${field.id}`}
							listeners={{
								onChange: ({ value }: { value: string }) =>
									updateScanSetting(field.id, Number(value))
							}}
						>
							{#snippet children(formField: AnyFieldApi)}
								<TextInput
									label={field.label}
									id={`scan_${field.id}`}
									field={formField}
									type="number"
									placeholder={field.placeholder ?? ''}
									disabled={readOnly}
									helpText={getHelpText(field)}
								/>
							{/snippet}
						</form.Field>
					{/if}
				{/each}
			</div>

			<!-- Port Scanning -->
			<div class="space-y-3">
				<h4 class="text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-gray-400">
					Port Scanning
				</h4>
				{#each portScanningFields as field (field.id)}
					{#if field.field_type === 'boolean'}
						<form.Field
							name={`scan_${field.id}`}
							listeners={{
								onChange: ({ value }: { value: boolean }) => updateScanSetting(field.id, value)
							}}
						>
							{#snippet children(formField: AnyFieldApi)}
								<Checkbox
									label={field.label}
									id={`scan_${field.id}`}
									field={formField}
									disabled={readOnly}
									helpText={getHelpText(field)}
								/>
							{/snippet}
						</form.Field>
					{:else}
						<form.Field
							name={`scan_${field.id}`}
							listeners={{
								onChange: ({ value }: { value: string }) =>
									updateScanSetting(field.id, Number(value))
							}}
						>
							{#snippet children(formField: AnyFieldApi)}
								<TextInput
									label={field.label}
									id={`scan_${field.id}`}
									field={formField}
									type="number"
									placeholder={field.placeholder ?? ''}
									disabled={readOnly}
									helpText={getHelpText(field)}
								/>
							{/snippet}
						</form.Field>
					{/if}
				{/each}
			</div>

			<!-- Advanced -->
			<div class="space-y-3">
				<h4 class="text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-gray-400">
					Advanced
				</h4>
				{#each advancedFields as field (field.id)}
					<form.Field
						name={`scan_${field.id}`}
						listeners={{
							onChange: ({ value }: { value: string }) => updateScanSetting(field.id, value)
						}}
					>
						{#snippet children(formField: AnyFieldApi)}
							<TextInput
								label={field.label}
								id={`scan_${field.id}`}
								field={formField}
								placeholder={field.placeholder ?? ''}
								disabled={readOnly}
								helpText={getHelpText(field)}
							/>
						{/snippet}
					</form.Field>
				{/each}
			</div>
		</div>
	{/if}
</div>

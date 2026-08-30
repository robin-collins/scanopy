<script lang="ts">
	import ConfigHeader from '$lib/shared/components/forms/config/ConfigHeader.svelte';
	import { pushError, pushSuccess } from '$lib/shared/stores/feedback';
	import type { Port } from '$lib/features/hosts/types/base';
	import {
		useClearHostPortOverrideMutation,
		useHostPortOverridesQuery,
		useUpsertHostPortOverrideMutation,
		type ServiceRefKind
	} from '$lib/features/host-port-overrides/queries';
	import { useServiceCatalogueQuery } from '$lib/features/services/queries';
	import {
		buildServiceReferenceOptions,
		decodeServiceReference,
		encodeServiceReference
	} from '$lib/features/host-port-overrides/service-reference';
	import {
		common_none,
		common_service,
		common_unknown,
		hosts_portOverrides_clear,
		hosts_portOverrides_cleared,
		hosts_portOverrides_displayNameLabel,
		hosts_portOverrides_displayNamePlaceholder,
		hosts_portOverrides_failedToClear,
		hosts_portOverrides_failedToSave,
		hosts_portOverrides_iconUrlLabel,
		hosts_portOverrides_iconUrlPlaceholder,
		hosts_portOverrides_noOverride,
		hosts_portOverrides_save,
		hosts_portOverrides_saved,
		hosts_portOverrides_subtitle,
		hosts_portOverrides_title
	} from '$lib/paraglide/messages';

	interface Props {
		port: Port;
		readOnly?: boolean;
	}

	let { port, readOnly = false }: Props = $props();

	let hostId = $derived(port.host_id);

	const overridesQuery = useHostPortOverridesQuery(() => hostId);
	const catalogueQuery = useServiceCatalogueQuery();
	const upsertMutation = useUpsertHostPortOverrideMutation();
	const clearMutation = useClearHostPortOverrideMutation();

	let displayName = $state('');
	let iconUrl = $state('');
	let serviceRefKind = $state<ServiceRefKind | null>(null);
	let serviceRefId = $state('');
	let dirty = $state(false);

	let override = $derived(
		(overridesQuery.data ?? []).find(
			(o) => o.port_number === port.number && o.port_protocol === port.protocol
		)
	);
	let selectedServiceValue = $derived(
		encodeServiceReference(
			serviceRefKind && serviceRefId ? { kind: serviceRefKind, id: serviceRefId } : null
		)
	);
	let serviceOptions = $derived(
		buildServiceReferenceOptions(
			catalogueQuery.data ?? [],
			serviceRefKind && serviceRefId ? { kind: serviceRefKind, id: serviceRefId } : null,
			common_unknown()
		)
	);

	$effect(() => {
		displayName = override?.display_name ?? '';
		iconUrl = override?.icon_url ?? '';
		serviceRefKind = override?.service_ref_kind ?? null;
		serviceRefId = override?.service_ref_id ?? '';
		dirty = false;
	});

	function handleDisplayNameChange(value: string) {
		displayName = value;
		dirty = true;
	}

	function handleIconUrlChange(value: string) {
		iconUrl = value;
		dirty = true;
	}

	function handleServiceChange(value: string) {
		const reference = decodeServiceReference(value);
		serviceRefKind = reference?.kind ?? null;
		serviceRefId = reference?.id ?? '';
		dirty = true;
	}

	function hasChanges() {
		return (
			dirty &&
			(displayName.trim() !== (override?.display_name ?? '').trim() ||
				iconUrl.trim() !== (override?.icon_url ?? '').trim() ||
				serviceRefKind !== (override?.service_ref_kind ?? null) ||
				serviceRefId !== (override?.service_ref_id ?? ''))
		);
	}

	async function handleSave() {
		const trimmedName = displayName.trim();
		const trimmedIcon = iconUrl.trim();
		try {
			await upsertMutation.mutateAsync({
				host_id: port.host_id,
				port_number: port.number,
				port_protocol: port.protocol,
				display_name: trimmedName.length > 0 ? trimmedName : null,
				icon_url: trimmedIcon.length > 0 ? trimmedIcon : null,
				service_ref_kind: serviceRefKind,
				service_ref_id: serviceRefKind ? serviceRefId : null
			});
			dirty = false;
			pushSuccess(hosts_portOverrides_saved());
		} catch (err) {
			pushError(hosts_portOverrides_failedToSave());
			console.error(err);
		}
	}

	async function handleClear() {
		try {
			await clearMutation.mutateAsync({
				host_id: port.host_id,
				port_number: port.number,
				port_protocol: port.protocol
			});
			dirty = false;
			pushSuccess(hosts_portOverrides_cleared());
		} catch (err) {
			pushError(hosts_portOverrides_failedToClear());
			console.error(err);
		}
	}
</script>

<div class="space-y-6">
	<ConfigHeader title={hosts_portOverrides_title()} subtitle={hosts_portOverrides_subtitle()} />

	{#if !override && !dirty}
		<p class="text-muted-foreground text-sm">{hosts_portOverrides_noOverride()}</p>
	{/if}

	<div class="space-y-4">
		<div class="space-y-1.5">
			<label for="port_override_service_{port.id}" class="text-sm font-medium">
				{common_service()}
			</label>
			<select
				id="port_override_service_{port.id}"
				class="input-field"
				value={selectedServiceValue}
				disabled={readOnly || catalogueQuery.isLoading}
				onchange={(e) => handleServiceChange(e.currentTarget.value)}
			>
				<option value="">{common_none()}</option>
				{#each serviceOptions as option (option.value)}
					<option value={option.value}>{option.label}</option>
				{/each}
			</select>
		</div>
		<div class="space-y-1.5">
			<label for="port_override_name_{port.id}" class="text-sm font-medium">
				{hosts_portOverrides_displayNameLabel()}
			</label>
			<input
				id="port_override_name_{port.id}"
				type="text"
				class="input-field"
				placeholder={hosts_portOverrides_displayNamePlaceholder()}
				value={displayName}
				disabled={readOnly}
				oninput={(e) => handleDisplayNameChange((e.currentTarget as HTMLInputElement).value)}
			/>
		</div>
		<div class="space-y-1.5">
			<label for="port_override_icon_{port.id}" class="text-sm font-medium">
				{hosts_portOverrides_iconUrlLabel()}
			</label>
			<input
				id="port_override_icon_{port.id}"
				type="url"
				class="input-field"
				placeholder={hosts_portOverrides_iconUrlPlaceholder()}
				value={iconUrl}
				disabled={readOnly}
				oninput={(e) => handleIconUrlChange((e.currentTarget as HTMLInputElement).value)}
			/>
		</div>
	</div>

	{#if !readOnly}
		<div class="flex gap-3">
			<button
				type="button"
				class="btn-primary"
				disabled={!hasChanges() || upsertMutation.isPending}
				onclick={handleSave}
			>
				{hosts_portOverrides_save()}
			</button>
			<button
				type="button"
				class="btn-secondary"
				disabled={!override || clearMutation.isPending}
				onclick={handleClear}
			>
				{hosts_portOverrides_clear()}
			</button>
		</div>
	{/if}
</div>

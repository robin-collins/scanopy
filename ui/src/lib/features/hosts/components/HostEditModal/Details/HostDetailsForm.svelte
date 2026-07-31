<script lang="ts">
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import type { HostFormData } from '$lib/features/hosts/types/base';
	import { hostnameFormat, max, required } from '$lib/shared/components/forms/validators';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import TextArea from '$lib/shared/components/forms/input/TextArea.svelte';
	import SelectNetwork from '$lib/features/networks/components/SelectNetwork.svelte';
	import SelectCategory from '$lib/features/categories/components/SelectCategory.svelte';
	import TagPicker from '$lib/features/tags/components/TagPicker.svelte';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import InfoRow from '$lib/shared/components/data/InfoRow.svelte';
	import { hostOsGroups } from '$lib/shared/stores/metadata';
	import type { HostOsGroup } from '$lib/features/hosts/types/base';
	import {
		common_contact,
		common_description,
		common_hostname,
		common_location,
		common_manufacturer,
		common_model,
		common_name,
		common_placeholderHostname,
		common_unassigned,
		hosts_details_descriptionPlaceholder,
		hosts_details_manufacturerPlaceholder,
		hosts_details_modelPlaceholder,
		hosts_details_namePlaceholder,
		hosts_details_osDetail,
		hosts_details_osDetailHelp,
		hosts_details_osDetailPlaceholder,
		hosts_details_osGroup,
		hosts_details_osGroupHelp,
		hosts_snmp_chassisId,
		hosts_snmp_managementUrl,
		hosts_snmp_sysDescr,
		hosts_snmp_sysObjectId,
		hosts_snmp_systemInfo
	} from '$lib/paraglide/messages';

	interface Props {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any };
		formData: HostFormData;
		isEditing?: boolean;
	}

	let { form, formData = $bindable(), isEditing = false }: Props = $props();

	// network_id is read/written directly against formData — no local
	// snapshot. A prior `$state(formData.network_id)` mirror captured the
	// value once at mount, went stale when HostEditor reassigned formData
	// via resetForm(host), and then got clobbered by SelectNetwork's
	// auto-default (first network) on the falsy initial capture.

	let osGroupOptions = $derived(
		hostOsGroups.getItems().map((g) => ({ value: g.id, label: g.name ?? g.id }))
	);

	function handleOsGroupChange(event: Event) {
		const value = (event.target as HTMLSelectElement).value;
		formData.os_group = (value || null) as HostOsGroup | null;
	}

	// Check if host has any SNMP system info
	let hasSnmpInfo = $derived(
		!!(
			formData.sys_descr ||
			formData.sys_object_id ||
			formData.sys_location ||
			formData.sys_contact ||
			formData.chassis_id ||
			formData.management_url
		)
	);
</script>

<div class="space-y-6 p-6">
	<div class="flex gap-6" class:flex-col={!isEditing || !hasSnmpInfo}>
		<!-- Form fields column -->
		<div class="min-w-0 space-y-6" class:flex-[3]={isEditing && hasSnmpInfo}>
			<div class="grid grid-cols-2 gap-6">
				<form.Field
					name="name"
					validators={{
						onBlur: ({ value }: { value: string }) => required(value) || max(100)(value)
					}}
				>
					{#snippet children(field: AnyFieldApi)}
						<TextInput
							label={common_name()}
							id="name"
							placeholder={hosts_details_namePlaceholder()}
							required={true}
							{field}
						/>
					{/snippet}
				</form.Field>

				<form.Field
					name="hostname"
					validators={{
						onBlur: ({ value }: { value: string }) => hostnameFormat(value)
					}}
				>
					{#snippet children(field: AnyFieldApi)}
						<TextInput
							label={common_hostname()}
							id="hostname"
							placeholder={common_placeholderHostname()}
							{field}
						/>
					{/snippet}
				</form.Field>
			</div>

			<SelectNetwork
				selectedNetworkId={formData.network_id}
				onNetworkChange={(id) => (formData.network_id = id)}
			/>

			<SelectCategory
				selectedCategoryId={formData.category_id}
				onCategoryChange={(id) => (formData.category_id = id)}
			/>

			<div class="grid grid-cols-2 gap-6">
				<div>
					<label for="manufacturer" class="text-secondary mb-2 block text-sm font-medium">
						{common_manufacturer()}
					</label>
					<input
						id="manufacturer"
						type="text"
						value={formData.manufacturer ?? ''}
						oninput={(e) => (formData.manufacturer = e.currentTarget.value || null)}
						placeholder={hosts_details_manufacturerPlaceholder()}
						class="input-field"
					/>
				</div>

				<div>
					<label for="model" class="text-secondary mb-2 block text-sm font-medium">
						{common_model()}
					</label>
					<input
						id="model"
						type="text"
						value={formData.model ?? ''}
						oninput={(e) => (formData.model = e.currentTarget.value || null)}
						placeholder={hosts_details_modelPlaceholder()}
						class="input-field"
					/>
				</div>
			</div>

			<div class="grid grid-cols-2 gap-6">
				<div>
					<label for="os_group" class="text-secondary mb-2 block text-sm font-medium">
						{hosts_details_osGroup()}
					</label>
					<select
						id="os_group"
						value={formData.os_group ?? ''}
						onchange={handleOsGroupChange}
						class="input-field"
					>
						<option class="select-option" value="">{common_unassigned()}</option>
						{#each osGroupOptions as option (option.value)}
							<option class="select-option" value={option.value}>{option.label}</option>
						{/each}
					</select>
					<p class="text-tertiary mt-2 text-xs">{hosts_details_osGroupHelp()}</p>
				</div>

				<div>
					<label for="os_detail" class="text-secondary mb-2 block text-sm font-medium">
						{hosts_details_osDetail()}
					</label>
					<input
						id="os_detail"
						type="text"
						value={formData.os_detail ?? ''}
						oninput={(e) => (formData.os_detail = e.currentTarget.value || null)}
						placeholder={hosts_details_osDetailPlaceholder()}
						class="input-field"
					/>
					<p class="text-tertiary mt-2 text-xs">{hosts_details_osDetailHelp()}</p>
				</div>
			</div>

			<form.Field
				name="description"
				validators={{
					onBlur: ({ value }: { value: string }) => max(500)(value)
				}}
			>
				{#snippet children(field: AnyFieldApi)}
					<TextArea
						label={common_description()}
						id="description"
						placeholder={hosts_details_descriptionPlaceholder()}
						{field}
					/>
				{/snippet}
			</form.Field>

			<TagPicker bind:selectedTagIds={formData.tags} />
		</div>

		<!-- SNMP System Info column (only when editing and has data) -->
		{#if isEditing && hasSnmpInfo}
			<div class="flex-[2]">
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
			</div>
		{/if}
	</div>
</div>

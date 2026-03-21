<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { validateForm } from '$lib/shared/components/forms/form-context';
	import {
		required,
		max,
		port,
		pemCertificate,
		pemPrivateKey
	} from '$lib/shared/components/forms/validators';
	import SegmentedControl from '$lib/shared/components/forms/SegmentedControl.svelte';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import { CredentialTypeDisplay } from '$lib/shared/components/forms/selection/display/CredentialTypeDisplay.svelte';
	import type { Credential, CredentialType } from '../types/base';
	import { createDefaultCredential } from '../types/base';
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { pushError } from '$lib/shared/stores/feedback';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import type { FieldDefinition } from '$lib/shared/stores/metadata';
	import { Eye, EyeOff } from 'lucide-svelte';
	import {
		common_couldNotLoadOrganization,
		common_create,
		common_delete,
		common_deleting,
		common_name,
		common_saving,
		common_update,
		credentials_credentialType,
		credentials_fileOnHost,
		credentials_filePathReadByDaemon,
		credentials_pasteValue,
		credentials_secretStoredInDatabase,
		credentials_typeImmutableWarning
	} from '$lib/paraglide/messages';

	interface Props {
		credential?: Credential | null;
		onSave: (data: Credential) => Promise<void>;
		onDelete?: ((id: string) => Promise<void>) | null;
		fixedCredentialType?: string;
		fixedName?: string;
		saveLabel?: string;
		compact?: boolean;
	}

	let {
		credential = null,
		onSave,
		onDelete = null,
		fixedCredentialType,
		fixedName,
		saveLabel: saveLabelOverride,
		compact = false
	}: Props = $props();

	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	let loading = $state(false);
	let deleting = $state(false);

	let isEditing = $derived(credential !== null);
	let defaultSaveLabel = $derived(isEditing ? common_update() : common_create());
	let saveLabel = $derived(saveLabelOverride ?? defaultSaveLabel);

	// Selected credential type ID for dynamic form rendering
	let selectedTypeId = $state<string>('SnmpV2c');

	// Dynamic field values keyed by field ID
	let fieldValues = $state<Record<string, string>>({});

	function getDefaultValues(): Credential {
		if (credential) return { ...credential };
		if (organization) return createDefaultCredential(organization.id);
		return createDefaultCredential('');
	}

	// Get field definitions for the currently selected type
	let currentFields: FieldDefinition[] = $derived.by(() => {
		const meta = credentialTypes.getMetadata(selectedTypeId);
		return meta?.fields ?? [];
	});

	// Group fields by their group property for visual grouping
	let fieldGroups = $derived.by(() => {
		const groups: { name: string | null; fields: FieldDefinition[] }[] = [];
		const groupOrder: (string | null)[] = [];
		const groupFields: Record<string, FieldDefinition[]> = {};
		const ungroupedFields: FieldDefinition[] = [];

		for (const field of currentFields) {
			const groupName = field.group ?? null;
			if (groupName === null) {
				if (!groupOrder.includes(null)) groupOrder.push(null);
				ungroupedFields.push(field);
			} else {
				if (!groupFields[groupName]) {
					groupFields[groupName] = [];
					groupOrder.push(groupName);
				}
				groupFields[groupName].push(field);
			}
		}

		for (const name of groupOrder) {
			if (name === null) {
				groups.push({ name: null, fields: ungroupedFields });
			} else {
				groups.push({ name, fields: groupFields[name] });
			}
		}
		return groups;
	});

	// Create form
	const form = createForm(() => ({
		defaultValues: createDefaultCredential(''),
		onSubmit: async ({ value }) => {
			if (!organization) {
				pushError(common_couldNotLoadOrganization());
				return;
			}

			if (!validateDynamicFields()) return;

			const credentialType = buildCredentialType();
			const nameValue = fixedName ?? (value as Credential).name?.trim() ?? '';

			const credentialData: Credential = {
				...(value as Credential),
				name: nameValue,
				organization_id: organization.id,
				credential_type: credentialType
			};

			loading = true;
			try {
				await onSave(credentialData);
			} finally {
				loading = false;
			}
		}
	}));

	function buildCredentialType(): CredentialType {
		const fields = currentFields;
		const typeObj: Record<string, unknown> = { type: selectedTypeId };

		for (const field of fields) {
			const value = fieldValues[field.id];
			if (field.field_type === 'secretpathorinline' || field.field_type === 'pathorinline') {
				if (field.optional && (!value || value.trim() === '')) {
					typeObj[field.id] = null;
				} else {
					try {
						typeObj[field.id] = JSON.parse(value);
					} catch {
						typeObj[field.id] = { mode: 'Inline', value };
					}
				}
			} else if (field.optional && (!value || value.trim() === '')) {
				typeObj[field.id] = null;
			} else {
				const raw = value ?? (field.default_value || '');
				const num = Number(raw);
				typeObj[field.id] = raw !== '' && !isNaN(num) && field.field_type === 'string' ? num : raw;
			}
		}

		return typeObj as unknown as CredentialType;
	}

	function initFieldValues(ct: CredentialType) {
		const values: Record<string, string> = {};
		const raw = ct as unknown as Record<string, unknown>;
		const fields = credentialTypes.getMetadata(raw.type as string)?.fields ?? [];
		const fieldMap = new Map(fields.map((f) => [f.id, f]));
		for (const [key, val] of Object.entries(raw)) {
			if (key === 'type') continue;
			const fieldDef = fieldMap.get(key);
			if (
				(fieldDef?.field_type === 'secretpathorinline' ||
					fieldDef?.field_type === 'pathorinline') &&
				val != null &&
				typeof val === 'object'
			) {
				values[key] = JSON.stringify(val);
			} else {
				values[key] = val != null ? String(val) : '';
			}
		}
		fieldValues = values;
	}

	function initDefaultFieldValues(typeId: string) {
		const meta = credentialTypes.getMetadata(typeId);
		const fields: FieldDefinition[] = meta?.fields ?? [];
		const values: Record<string, string> = {};
		for (const field of fields) {
			if (field.field_type === 'pathorinline') {
				values[field.id] = JSON.stringify({ mode: 'Inline', value: '' });
			} else {
				values[field.id] = field.default_value ?? '';
			}
		}
		fieldValues = values;
	}

	let secretFieldModes = $state<Record<string, 'inline' | 'filepath'>>({});
	let fileFieldModes = $state<Record<string, 'inline' | 'filepath'>>({});
	let secretFieldVisible = $state<Record<string, boolean>>({});
	let fieldErrors = $state<Record<string, string | undefined>>({});

	// Initialize on mount or when credential changes
	export function reset() {
		const defaults = getDefaultValues();
		form.reset(defaults);
		secretFieldModes = {};
		fileFieldModes = {};
		secretFieldVisible = {};
		fieldErrors = {};

		if (credential) {
			selectedTypeId = credential.credential_type.type;
			initFieldValues(credential.credential_type);
			const raw = credential.credential_type as unknown as Record<string, unknown>;
			const fields = credentialTypes.getMetadata(selectedTypeId)?.fields ?? [];
			const fieldMap = new Map(fields.map((f) => [f.id, f]));
			for (const [key, val] of Object.entries(raw)) {
				if (val && typeof val === 'object' && 'mode' in (val as Record<string, unknown>)) {
					const sv = val as { mode: string };
					const mode = sv.mode === 'FilePath' ? 'filepath' : 'inline';
					const fieldDef = fieldMap.get(key);
					if (fieldDef?.field_type === 'pathorinline') {
						fileFieldModes[key] = mode;
					} else {
						secretFieldModes[key] = mode;
					}
				}
			}
		} else {
			const typeId = fixedCredentialType ?? 'SnmpV2c';
			selectedTypeId = typeId;
			initDefaultFieldValues(typeId);
		}

		// Set fixed name if provided
		if (fixedName) {
			form.setFieldValue('name', fixedName);
		}
	}

	// Initialize on mount (called once, not reactively)
	reset();

	function handleTypeChange(typeId: string) {
		selectedTypeId = typeId;
		initDefaultFieldValues(selectedTypeId);
		fieldErrors = {};
	}

	async function handleSubmit() {
		const isValid = await validateForm(form, new Set(['name']));
		if (!isValid) return;
		await form.handleSubmit();
	}

	async function handleDelete() {
		if (onDelete && credential) {
			deleting = true;
			try {
				await onDelete(credential.id);
			} finally {
				deleting = false;
			}
		}
	}

	let typeOptions = $derived(credentialTypes.getItems());

	function getSecretFieldMode(fieldId: string): 'inline' | 'filepath' {
		return secretFieldModes[fieldId] ?? 'inline';
	}

	function setSecretFieldMode(fieldId: string, mode: 'inline' | 'filepath') {
		secretFieldModes[fieldId] = mode;
		const current = fieldValues[fieldId];
		let parsed: { mode?: string; value?: string; path?: string };
		try {
			parsed = current ? JSON.parse(current) : {};
		} catch {
			parsed = {};
		}
		if (mode === 'inline') {
			fieldValues[fieldId] = JSON.stringify({
				mode: 'Inline',
				value: parsed.value ?? parsed.path ?? ''
			});
		} else {
			fieldValues[fieldId] = JSON.stringify({
				mode: 'FilePath',
				path: parsed.path ?? parsed.value ?? ''
			});
		}
		fieldErrors[fieldId] = undefined;
	}

	function getSecretFieldDisplayValue(fieldId: string): string {
		const raw = fieldValues[fieldId];
		if (!raw) return '';
		try {
			const parsed = JSON.parse(raw);
			if (parsed.mode === 'Inline') return parsed.value ?? '';
			if (parsed.mode === 'FilePath') return parsed.path ?? '';
		} catch {
			// not JSON yet
		}
		return raw;
	}

	function setSecretFieldDisplayValue(fieldId: string, displayValue: string) {
		const mode = getSecretFieldMode(fieldId);
		if (mode === 'inline') {
			fieldValues[fieldId] = JSON.stringify({ mode: 'Inline', value: displayValue });
		} else {
			fieldValues[fieldId] = JSON.stringify({ mode: 'FilePath', path: displayValue });
		}
	}

	function getFileFieldMode(fieldId: string): 'inline' | 'filepath' {
		return fileFieldModes[fieldId] ?? 'inline';
	}

	function setFileFieldMode(fieldId: string, mode: 'inline' | 'filepath') {
		fileFieldModes[fieldId] = mode;
		const current = fieldValues[fieldId];
		let parsed: { mode?: string; value?: string; path?: string };
		try {
			parsed = current ? JSON.parse(current) : {};
		} catch {
			parsed = {};
		}
		if (mode === 'inline') {
			fieldValues[fieldId] = JSON.stringify({
				mode: 'Inline',
				value: parsed.value ?? parsed.path ?? ''
			});
		} else {
			fieldValues[fieldId] = JSON.stringify({
				mode: 'FilePath',
				path: parsed.path ?? parsed.value ?? ''
			});
		}
		fieldErrors[fieldId] = undefined;
	}

	function getFileFieldDisplayValue(fieldId: string): string {
		const raw = fieldValues[fieldId];
		if (!raw) return '';
		try {
			const parsed = JSON.parse(raw);
			if (parsed.mode === 'Inline') return parsed.value ?? '';
			if (parsed.mode === 'FilePath') return parsed.path ?? '';
		} catch {
			// not JSON yet
		}
		return raw;
	}

	function setFileFieldDisplayValue(fieldId: string, displayValue: string) {
		const mode = getFileFieldMode(fieldId);
		if (mode === 'inline') {
			fieldValues[fieldId] = JSON.stringify({ mode: 'Inline', value: displayValue });
		} else {
			fieldValues[fieldId] = JSON.stringify({ mode: 'FilePath', path: displayValue });
		}
	}

	function validateDynamicFields(): boolean {
		const errors: Record<string, string | undefined> = {};
		let valid = true;

		for (const field of currentFields) {
			const value = fieldValues[field.id];

			if (!field.optional) {
				if (field.field_type === 'secretpathorinline') {
					const displayVal = getSecretFieldDisplayValue(field.id);
					if (!displayVal || displayVal.trim() === '') {
						errors[field.id] = 'This field is required';
						valid = false;
						continue;
					}
				} else if (field.field_type === 'pathorinline') {
					const displayVal = getFileFieldDisplayValue(field.id);
					if (!displayVal || displayVal.trim() === '') {
						errors[field.id] = 'This field is required';
						valid = false;
						continue;
					}
				} else if (!value || value.trim() === '') {
					errors[field.id] = 'This field is required';
					valid = false;
					continue;
				}
			}

			if (field.id === 'port' || field.label?.toLowerCase().includes('port')) {
				if (value && value.trim() !== '') {
					const portError = port(value);
					if (portError) {
						errors[field.id] = portError;
						valid = false;
						continue;
					}
				}
			}

			if (field.field_type === 'secretpathorinline') {
				const mode = getSecretFieldMode(field.id);
				if (mode === 'inline' && field.inline_format === 'pemprivatekey') {
					const displayVal = getSecretFieldDisplayValue(field.id);
					if (displayVal && displayVal !== '********') {
						const error = pemPrivateKey(displayVal);
						if (error) {
							errors[field.id] = error;
							valid = false;
						}
					}
				}
			} else if (field.field_type === 'pathorinline') {
				const mode = getFileFieldMode(field.id);
				if (mode === 'inline' && field.inline_format === 'pemcertificate') {
					const displayVal = getFileFieldDisplayValue(field.id);
					if (displayVal && displayVal.trim() !== '') {
						const error = pemCertificate(displayVal);
						if (error) {
							errors[field.id] = error;
							valid = false;
						}
					}
				}
			}
		}

		fieldErrors = errors;
		return valid;
	}

	function validateFieldOnBlur(field: FieldDefinition) {
		const value = fieldValues[field.id];
		const errors = { ...fieldErrors };

		if (!field.optional && field.field_type !== 'secretpathorinline') {
			if (!value || value.trim() === '') {
				errors[field.id] = 'This field is required';
				fieldErrors = errors;
				return;
			}
		}

		if (field.id === 'port' || field.label?.toLowerCase().includes('port')) {
			if (value && value.trim() !== '') {
				const portError = port(value);
				if (portError) {
					errors[field.id] = portError;
					fieldErrors = errors;
					return;
				}
			}
		}

		errors[field.id] = undefined;
		fieldErrors = errors;
	}

	// Whether to show type selector and name field
	let showTypeSelector = $derived(!fixedCredentialType && !isEditing);
	let showName = $derived(!fixedName);
</script>

<form
	onsubmit={(e) => {
		e.preventDefault();
		e.stopPropagation();
		handleSubmit();
	}}
	class="flex flex-col gap-4"
>
	{#if compact}
		<!-- Compact: fields directly, no card wrapper -->
		<div class="space-y-4">
			{#if showName}
				<form.Field
					name="name"
					validators={{
						onBlur: ({ value }) => required(value) || max(100)(value)
					}}
				>
					{#snippet children(field)}
						<TextInput
							label={common_name()}
							id="credential-name"
							{field}
							placeholder="e.g. Docker Proxy"
							required
						/>
					{/snippet}
				</form.Field>
			{/if}

			{#if showTypeSelector}
				<div class="space-y-2">
					<RichSelect
						label={credentials_credentialType()}
						selectedValue={selectedTypeId}
						options={typeOptions}
						displayComponent={CredentialTypeDisplay}
						disabled={isEditing}
						onSelect={handleTypeChange}
					/>
					{#if !isEditing}
						<p class="text-muted mt-1 text-xs">{credentials_typeImmutableWarning()}</p>
					{/if}
				</div>
			{/if}

			{#each fieldGroups as group (group.name ?? '_ungrouped')}
				{#if group.name}
					<InfoCard title={group.name}>
						{#each group.fields as field (field.id)}
							{@render fieldRenderer(field, field.secret)}
						{/each}
					</InfoCard>
				{:else}
					{#each group.fields as field (field.id)}
						{@render fieldRenderer(field, field.secret)}
					{/each}
				{/if}
			{/each}
		</div>
	{:else}
		<!-- Standard: separate cards -->
		<div class="card card-static space-y-4 p-4">
			{#if showName}
				<form.Field
					name="name"
					validators={{
						onBlur: ({ value }) => required(value) || max(100)(value)
					}}
				>
					{#snippet children(field)}
						<TextInput
							label={common_name()}
							id="credential-name"
							{field}
							placeholder="e.g. Office SNMP"
							required
						/>
					{/snippet}
				</form.Field>
			{/if}

			{#if showTypeSelector}
				<div class="space-y-2">
					<RichSelect
						label={credentials_credentialType()}
						selectedValue={selectedTypeId}
						options={typeOptions}
						displayComponent={CredentialTypeDisplay}
						disabled={isEditing}
						onSelect={handleTypeChange}
					/>
					{#if !isEditing}
						<p class="text-muted mt-1 text-xs">{credentials_typeImmutableWarning()}</p>
					{/if}
				</div>
			{/if}
		</div>

		{#each fieldGroups as group (group.name ?? '_ungrouped')}
			{#if group.name}
				<InfoCard title={group.name}>
					{#each group.fields as field (field.id)}
						{@render fieldRenderer(field, field.secret)}
					{/each}
				</InfoCard>
			{:else if group.fields.length > 0}
				<div class="card card-static space-y-4 p-4">
					{#each group.fields as field (field.id)}
						{@render fieldRenderer(field, field.secret)}
					{/each}
				</div>
			{/if}
		{/each}
	{/if}

	{#if !compact}
		<!-- Footer -->
		<div class="flex items-center justify-between">
			<div>
				{#if isEditing && onDelete && credential}
					<button
						type="button"
						disabled={deleting || loading}
						onclick={handleDelete}
						class="btn-danger"
					>
						{deleting ? common_deleting() : common_delete()}
					</button>
				{/if}
			</div>
			<button type="submit" disabled={loading || deleting} class="btn-primary">
				{loading ? common_saving() : saveLabel}
			</button>
		</div>
	{/if}
</form>

{#snippet fieldRenderer(field: FieldDefinition, isSecret: boolean)}
	<div class="space-y-1">
		<label for={field.id} class="text-secondary block text-sm font-medium">
			{field.label}
			{#if !field.optional}
				<span class="text-red-400">*</span>
			{/if}
		</label>

		{#if field.field_type === 'select'}
			<select
				id={field.id}
				value={fieldValues[field.id] ?? field.default_value ?? ''}
				onchange={(e) => {
					const target = e.target as HTMLSelectElement;
					fieldValues[field.id] = target.value;
				}}
				class="select-trigger text-primary w-full rounded-md px-3 py-2 text-sm"
				class:input-field-error={!!fieldErrors[field.id]}
			>
				{#each field.options ?? [] as option (option)}
					<option value={option}>{option}</option>
				{/each}
			</select>
		{:else if field.field_type === 'secretpathorinline'}
			<div class="space-y-2">
				<SegmentedControl
					options={[
						{ value: 'inline', label: credentials_pasteValue() },
						{ value: 'filepath', label: credentials_fileOnHost() }
					]}
					selected={getSecretFieldMode(field.id)}
					onchange={(v) => setSecretFieldMode(field.id, v as 'inline' | 'filepath')}
					size="sm"
				/>
				{#if getSecretFieldMode(field.id) === 'inline'}
					<p class="text-muted text-xs">
						{credentials_secretStoredInDatabase()}
					</p>
					{#if field.inline_format === 'pemprivatekey' || !field.inline_format}
						<div class="relative">
							<textarea
								id={field.id}
								value={getSecretFieldDisplayValue(field.id)}
								oninput={(e) => {
									const target = e.target as HTMLTextAreaElement;
									setSecretFieldDisplayValue(field.id, target.value);
								}}
								placeholder={field.placeholder ?? '-----BEGIN PRIVATE KEY-----'}
								rows={4}
								class="input-field text-primary w-full rounded-md px-3 py-2 pr-10 font-mono text-sm"
								class:password-field={!secretFieldVisible[field.id]}
								class:input-field-error={!!fieldErrors[field.id]}
							></textarea>
							{#if getSecretFieldDisplayValue(field.id) && getSecretFieldDisplayValue(field.id) !== '********'}
								<button
									type="button"
									class="text-muted hover:text-secondary absolute right-2 top-2"
									onclick={() => (secretFieldVisible[field.id] = !secretFieldVisible[field.id])}
								>
									{#if secretFieldVisible[field.id]}
										<EyeOff class="h-4 w-4" />
									{:else}
										<Eye class="h-4 w-4" />
									{/if}
								</button>
							{/if}
						</div>
					{:else}
						<div class="relative">
							<input
								id={field.id}
								type={secretFieldVisible[field.id] ? 'text' : 'password'}
								value={getSecretFieldDisplayValue(field.id)}
								oninput={(e) => {
									const target = e.target as HTMLInputElement;
									setSecretFieldDisplayValue(field.id, target.value);
								}}
								placeholder={field.placeholder ?? ''}
								class="input-field text-primary w-full rounded-md px-3 py-2 pr-10 text-sm"
								class:input-field-error={!!fieldErrors[field.id]}
							/>
							{#if getSecretFieldDisplayValue(field.id) && getSecretFieldDisplayValue(field.id) !== '********'}
								<button
									type="button"
									class="text-muted hover:text-secondary absolute right-2 top-1/2 -translate-y-1/2"
									onclick={() => (secretFieldVisible[field.id] = !secretFieldVisible[field.id])}
								>
									{#if secretFieldVisible[field.id]}
										<EyeOff class="h-4 w-4" />
									{:else}
										<Eye class="h-4 w-4" />
									{/if}
								</button>
							{/if}
						</div>
					{/if}
				{:else}
					<p class="text-muted text-xs">
						{credentials_filePathReadByDaemon()}
					</p>
					<input
						id={field.id}
						type="text"
						value={getSecretFieldDisplayValue(field.id)}
						oninput={(e) => {
							const target = e.target as HTMLInputElement;
							setSecretFieldDisplayValue(field.id, target.value);
						}}
						placeholder="/etc/docker/certs/key.pem"
						class="input-field text-primary w-full rounded-md px-3 py-2 text-sm"
						class:input-field-error={!!fieldErrors[field.id]}
					/>
				{/if}
			</div>
		{:else if field.field_type === 'pathorinline'}
			<div class="space-y-2">
				<SegmentedControl
					options={[
						{ value: 'inline', label: credentials_pasteValue() },
						{ value: 'filepath', label: credentials_fileOnHost() }
					]}
					selected={getFileFieldMode(field.id)}
					onchange={(v) => setFileFieldMode(field.id, v as 'inline' | 'filepath')}
					size="sm"
				/>
				{#if getFileFieldMode(field.id) === 'inline'}
					<p class="text-muted text-xs">
						{credentials_secretStoredInDatabase()}
					</p>
					{#if field.inline_format === 'pemcertificate'}
						<textarea
							id={field.id}
							value={getFileFieldDisplayValue(field.id)}
							oninput={(e) => {
								const target = e.target as HTMLTextAreaElement;
								setFileFieldDisplayValue(field.id, target.value);
							}}
							placeholder={field.placeholder ?? ''}
							rows={4}
							class="input-field text-primary w-full rounded-md px-3 py-2 font-mono text-sm"
							class:input-field-error={!!fieldErrors[field.id]}
						></textarea>
					{:else}
						<input
							id={field.id}
							type="text"
							value={getFileFieldDisplayValue(field.id)}
							oninput={(e) => {
								const target = e.target as HTMLInputElement;
								setFileFieldDisplayValue(field.id, target.value);
							}}
							placeholder={field.placeholder ?? ''}
							class="input-field text-primary w-full rounded-md px-3 py-2 text-sm"
							class:input-field-error={!!fieldErrors[field.id]}
						/>
					{/if}
				{:else}
					<p class="text-muted text-xs">
						{credentials_filePathReadByDaemon()}
					</p>
					<input
						id={field.id}
						type="text"
						value={getFileFieldDisplayValue(field.id)}
						oninput={(e) => {
							const target = e.target as HTMLInputElement;
							setFileFieldDisplayValue(field.id, target.value);
						}}
						placeholder="/etc/docker/certs/cert.pem"
						class="input-field text-primary w-full rounded-md px-3 py-2 text-sm"
						class:input-field-error={!!fieldErrors[field.id]}
					/>
				{/if}
			</div>
		{:else if field.field_type === 'text'}
			<textarea
				id={field.id}
				value={fieldValues[field.id] ?? ''}
				oninput={(e) => {
					const target = e.target as HTMLTextAreaElement;
					fieldValues[field.id] = target.value;
				}}
				onblur={() => validateFieldOnBlur(field)}
				placeholder={field.placeholder ?? ''}
				rows={4}
				class="input-field text-primary w-full rounded-md px-3 py-2 font-mono text-sm"
				class:password-field={isSecret}
				class:input-field-error={!!fieldErrors[field.id]}
			></textarea>
		{:else}
			<input
				id={field.id}
				type={isSecret ? 'password' : 'text'}
				value={fieldValues[field.id] ?? ''}
				oninput={(e) => {
					const target = e.target as HTMLInputElement;
					fieldValues[field.id] = target.value;
				}}
				onblur={() => validateFieldOnBlur(field)}
				placeholder={field.placeholder ?? ''}
				class="input-field text-primary w-full rounded-md px-3 py-2 text-sm"
				class:input-field-error={!!fieldErrors[field.id]}
			/>
		{/if}

		{#if field.help_text}
			<p class="text-muted text-xs">{field.help_text}</p>
		{/if}
		{#if fieldErrors[field.id]}
			<p class="text-xs text-red-400">{fieldErrors[field.id]}</p>
		{/if}
	</div>
{/snippet}

<style>
	.password-field {
		-webkit-text-security: disc;
	}
</style>

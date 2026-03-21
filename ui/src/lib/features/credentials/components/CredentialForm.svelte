<script lang="ts">
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import { createForm } from '@tanstack/svelte-form';
	import { validateForm } from '$lib/shared/components/forms/form-context';
	import {
		required,
		max,
		port,
		pemCertificate,
		pemPrivateKey,
		ipAddressFormat
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
	import { submitForm } from '$lib/shared/components/forms/form-context';
	// submitForm used by handleSubmit for standard mode form
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import type { FieldDefinition } from '$lib/shared/stores/metadata';
	import { Eye, EyeOff } from 'lucide-svelte';
	import {
		common_couldNotLoadOrganization,
		common_name,
		credentials_credentialType,
		credentials_fileOnHost,
		credentials_filePathReadByDaemon,
		credentials_pasteValue,
		credentials_secretStoredInDatabase,
		credentials_typeImmutableWarning,
		daemons_credentialWizardSeedIp,
		daemons_credentialWizardSeedIpHelp,
		daemons_credentialWizardTargetDaemonHost,
		daemons_credentialWizardDaemonHostHelp,
		common_ipAddress,
		common_target
	} from '$lib/paraglide/messages';

	interface Props {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: any;
		credential?: Credential | null;
		fixedCredentialType?: string;
		fixedName?: string;
		compact?: boolean;
		hideFields?: boolean;
		fieldPrefix?: string;
		onChange?: (data: { seedIp?: string; fieldValues?: Record<string, string> }) => void;
	}

	let {
		form,
		credential = null,
		fixedCredentialType,
		fixedName,
		compact = false,
		hideFields = false,
		fieldPrefix = '',
		onChange
	}: Props = $props();

	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	let isEditing = $derived(credential !== null);

	// Selected credential type ID for dynamic form rendering
	let selectedTypeId = $state<string>('SnmpV2c');

	// Dynamic field values keyed by field ID
	let fieldValues = $state<Record<string, string>>({});

	// Target mode: 'ip' for manual IP entry, 'daemon_host' for localhost
	let targetMode = $state<'ip' | 'daemon_host'>('ip');

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

	// --- Secret/file field mode tracking ---
	let secretFieldModes = $state<Record<string, 'inline' | 'filepath'>>({});
	let fileFieldModes = $state<Record<string, 'inline' | 'filepath'>>({});
	let secretFieldVisible = $state<Record<string, boolean>>({});

	function getDefaultValues(): Credential {
		if (credential) return { ...credential };
		if (organization) return createDefaultCredential(organization.id);
		return createDefaultCredential('');
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

	export function reset() {
		const defaults = getDefaultValues();
		secretFieldModes = {};
		fileFieldModes = {};
		secretFieldVisible = {};
		targetMode = 'ip';

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
			// Reset form fields for modal mode
			if (!compact) {
				form.setFieldValue?.('name', defaults.name);
			}
		} else {
			const typeId = fixedCredentialType ?? 'SnmpV2c';
			selectedTypeId = typeId;
			initDefaultFieldValues(typeId);
		}

		// Set fixed name if provided
		if (fixedName && !compact) {
			form.setFieldValue?.('name', fixedName);
		}

		// Initialize target mode from the form's seed IP value
		if (compact) {
			const formSeedIp = form.getFieldValue?.(`${fieldPrefix}seedIp`) as string | undefined;
			if (formSeedIp === '127.0.0.1' || formSeedIp === '::1') {
				targetMode = 'daemon_host';
			}
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

	/** Build a CredentialType from current fieldValues. */
	export function buildCredentialType(): CredentialType {
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

	function handleTypeChange(typeId: string) {
		selectedTypeId = typeId;
		initDefaultFieldValues(selectedTypeId);
	}

	async function handleSubmit() {
		if (!organization) {
			pushError(common_couldNotLoadOrganization());
			return;
		}

		await submitForm(form);
	}

	let typeOptions = $derived(credentialTypes.getItems());

	// Whether to show type selector and name field
	let showTypeSelector = $derived(!fixedCredentialType);
	let showName = $derived(!fixedName && !compact);

	// --- Field name helpers ---
	function fieldName(id: string): string {
		return `${fieldPrefix}fields.${id}`;
	}

	let seedIpFieldName = $derived(`${fieldPrefix}seedIp`);
	let nameFieldName = $derived(`${fieldPrefix}name`);

	// --- Secret/file field helpers ---
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
		onChange?.({ fieldValues: { ...fieldValues } });
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
		onChange?.({ fieldValues: { ...fieldValues } });
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
		onChange?.({ fieldValues: { ...fieldValues } });
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
		onChange?.({ fieldValues: { ...fieldValues } });
	}

	function handleTargetModeChange(mode: 'ip' | 'daemon_host') {
		targetMode = mode;
		if (mode === 'daemon_host') {
			handleSeedIpChange('127.0.0.1');
			form.setFieldValue?.(`${fieldPrefix}seedIp`, '127.0.0.1');
		} else {
			handleSeedIpChange('');
			form.setFieldValue?.(`${fieldPrefix}seedIp`, '');
		}
	}

	function handleFieldValueChange(fieldId: string, value: string) {
		fieldValues[fieldId] = value;
		onChange?.({ fieldValues: { ...fieldValues } });
	}

	function handleSeedIpChange(value: string) {
		onChange?.({ seedIp: value });
	}

	// Build validators for a credential field based on its definition
	function getFieldValidators(field: FieldDefinition) {
		const validate = ({ value }: { value: string }) => {
			// For path-or-inline fields, check the actual display value, not the JSON wrapper
			let effectiveValue = value;
			if (field.field_type === 'secretpathorinline' || field.field_type === 'pathorinline') {
				if (field.field_type === 'secretpathorinline') {
					effectiveValue = getSecretFieldDisplayValue(field.id);
				} else {
					effectiveValue = getFileFieldDisplayValue(field.id);
				}
			}
			if (!field.optional && !effectiveValue?.trim()) return 'This field is required';
			// Skip all further validation if value is empty (optional field)
			if (!effectiveValue?.trim()) return undefined;
			if (field.id === 'port' || field.label?.toLowerCase().includes('port')) {
				return port(effectiveValue);
			}
			// Only validate PEM format when in inline mode
			if (field.field_type === 'secretpathorinline') {
				if (getSecretFieldMode(field.id) !== 'inline') return undefined;
			}
			if (field.field_type === 'pathorinline') {
				if (getFileFieldMode(field.id) !== 'inline') return undefined;
			}
			if (field.inline_format === 'pemprivatekey' && effectiveValue !== '********') {
				return pemPrivateKey(effectiveValue);
			}
			if (field.inline_format === 'pemcertificate') {
				return pemCertificate(effectiveValue);
			}
			return undefined;
		};
		return { onBlur: validate, onSubmit: validate };
	}
</script>

{#if compact}
	<div class="space-y-4">
		<!-- Target mode selector (compact mode only) -->
		<div class="space-y-2">
			<label class="text-secondary block text-sm font-medium">{common_target()}</label>
			<SegmentedControl
				options={[
					{ value: 'ip', label: common_ipAddress() },
					{ value: 'daemon_host', label: daemons_credentialWizardTargetDaemonHost() }
				]}
				selected={targetMode}
				onchange={(v) => handleTargetModeChange(v as 'ip' | 'daemon_host')}
				size="sm"
			/>
		</div>

		{#if targetMode === 'ip'}
			<form.Field
				name={seedIpFieldName}
				validators={{
					onBlur: ({ value }: { value: string }) => required(value) || ipAddressFormat(value),
					onChange: ({ value }: { value: string }) => required(value) || ipAddressFormat(value),
					onSubmit: ({ value }: { value: string }) => required(value) || ipAddressFormat(value)
				}}
				listeners={{
					onChange: ({ value }: { value: string }) => handleSeedIpChange(value)
				}}
			>
				{#snippet children(field: AnyFieldApi)}
					<TextInput
						label={daemons_credentialWizardSeedIp()}
						id="seed-ip-{fieldPrefix}"
						placeholder="e.g. 192.168.1.1"
						helpText={daemons_credentialWizardSeedIpHelp()}
						required={true}
						{field}
					/>
				{/snippet}
			</form.Field>
		{:else}
			<p class="text-muted text-xs">{daemons_credentialWizardDaemonHostHelp()}</p>
		{/if}

		<!-- Credential fields -->
		{#if !hideFields}
			{#each fieldGroups as group (group.name ?? '_ungrouped')}
				{#if group.name}
					<InfoCard title={group.name}>
						{#each group.fields as field (field.id)}
							{@render fieldRenderer(field, field.secret)}
						{/each}
					</InfoCard>
				{:else if group.fields.length > 0}
					<InfoCard title={null}>
						{#each group.fields as field (field.id)}
							{@render fieldRenderer(field, field.secret)}
						{/each}
					</InfoCard>
				{/if}
			{/each}
		{/if}
	</div>
{:else}
	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			handleSubmit();
		}}
		class="flex flex-col gap-4"
	>
		<!-- Standard mode: card wrapper for name/type, separate cards for fields -->
		<div class="card card-static space-y-4 p-4">
			{#if showName}
				<form.Field
					name={nameFieldName}
					validators={{
						onBlur: ({ value }: { value: string }) => required(value) || max(100)(value),
						onSubmit: ({ value }: { value: string }) => required(value) || max(100)(value)
					}}
				>
					{#snippet children(field: AnyFieldApi)}
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

		<!-- Hidden submit button for Enter-to-submit -->
		<button type="submit" class="hidden"></button>
	</form>
{/if}

{#snippet fieldRenderer(field: FieldDefinition, isSecret: boolean)}
	{@const fName = fieldName(field.id)}
	<div class="space-y-1">
		{#if field.field_type === 'select'}
			<form.Field name={fName} validators={getFieldValidators(field)}>
				{#snippet children(formField: AnyFieldApi)}
					<label for={field.id} class="text-secondary block text-sm font-medium">
						{field.label}
						{#if !field.optional}
							<span class="text-red-400">*</span>
						{/if}
					</label>
					<select
						id={field.id}
						value={fieldValues[field.id] ?? field.default_value ?? ''}
						onchange={(e) => {
							const target = e.target as HTMLSelectElement;
							handleFieldValueChange(field.id, target.value);
							formField.handleChange(target.value);
						}}
						onblur={() => formField.handleBlur()}
						class="select-trigger text-primary w-full rounded-md px-3 py-2 text-sm"
						class:input-field-error={formField.state.meta.errors?.length > 0}
					>
						{#each field.options ?? [] as option (option)}
							<option value={option}>{option}</option>
						{/each}
					</select>
				{/snippet}
			</form.Field>
		{:else if field.field_type === 'secretpathorinline'}
			<form.Field name={fName} validators={getFieldValidators(field)}>
				{#snippet children(formField: AnyFieldApi)}
					<label for={field.id} class="text-secondary block text-sm font-medium">
						{field.label}
						{#if !field.optional}
							<span class="text-red-400">*</span>
						{/if}
					</label>
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
											formField.handleChange(target.value);
										}}
										onblur={() => formField.handleBlur()}
										placeholder={field.placeholder ?? '-----BEGIN PRIVATE KEY-----'}
										rows={4}
										class="input-field text-primary w-full rounded-md px-3 py-2 pr-10 font-mono text-sm"
										class:password-field={!secretFieldVisible[field.id]}
										class:input-field-error={formField.state.meta.errors?.length > 0}
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
											formField.handleChange(target.value);
										}}
										onblur={() => formField.handleBlur()}
										placeholder={field.placeholder ?? ''}
										class="input-field text-primary w-full rounded-md px-3 py-2 pr-10 text-sm"
										class:input-field-error={formField.state.meta.errors?.length > 0}
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
									formField.handleChange(target.value);
								}}
								onblur={() => formField.handleBlur()}
								placeholder="/etc/docker/certs/key.pem"
								class="input-field text-primary w-full rounded-md px-3 py-2 text-sm"
								class:input-field-error={formField.state.meta.errors?.length > 0}
							/>
						{/if}
					</div>
				{/snippet}
			</form.Field>
		{:else if field.field_type === 'pathorinline'}
			<form.Field name={fName} validators={getFieldValidators(field)}>
				{#snippet children(formField: AnyFieldApi)}
					<label for={field.id} class="text-secondary block text-sm font-medium">
						{field.label}
						{#if !field.optional}
							<span class="text-red-400">*</span>
						{/if}
					</label>
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
										formField.handleChange(target.value);
									}}
									onblur={() => formField.handleBlur()}
									placeholder={field.placeholder ?? ''}
									rows={4}
									class="input-field text-primary w-full rounded-md px-3 py-2 font-mono text-sm"
									class:input-field-error={formField.state.meta.errors?.length > 0}
								></textarea>
							{:else}
								<input
									id={field.id}
									type="text"
									value={getFileFieldDisplayValue(field.id)}
									oninput={(e) => {
										const target = e.target as HTMLInputElement;
										setFileFieldDisplayValue(field.id, target.value);
										formField.handleChange(target.value);
									}}
									onblur={() => formField.handleBlur()}
									placeholder={field.placeholder ?? ''}
									class="input-field text-primary w-full rounded-md px-3 py-2 text-sm"
									class:input-field-error={formField.state.meta.errors?.length > 0}
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
									formField.handleChange(target.value);
								}}
								onblur={() => formField.handleBlur()}
								placeholder="/etc/docker/certs/cert.pem"
								class="input-field text-primary w-full rounded-md px-3 py-2 text-sm"
								class:input-field-error={formField.state.meta.errors?.length > 0}
							/>
						{/if}
					</div>
				{/snippet}
			</form.Field>
		{:else if field.field_type === 'text'}
			<form.Field name={fName} validators={getFieldValidators(field)}>
				{#snippet children(formField: AnyFieldApi)}
					<label for={field.id} class="text-secondary block text-sm font-medium">
						{field.label}
						{#if !field.optional}
							<span class="text-red-400">*</span>
						{/if}
					</label>
					<textarea
						id={field.id}
						value={fieldValues[field.id] ?? ''}
						oninput={(e) => {
							const target = e.target as HTMLTextAreaElement;
							handleFieldValueChange(field.id, target.value);
							formField.handleChange(target.value);
						}}
						onblur={() => formField.handleBlur()}
						placeholder={field.placeholder ?? ''}
						rows={4}
						class="input-field text-primary w-full rounded-md px-3 py-2 font-mono text-sm"
						class:password-field={isSecret}
						class:input-field-error={formField.state.meta.errors?.length > 0}
					></textarea>
				{/snippet}
			</form.Field>
		{:else}
			<form.Field name={fName} validators={getFieldValidators(field)}>
				{#snippet children(formField: AnyFieldApi)}
					<label for={field.id} class="text-secondary block text-sm font-medium">
						{field.label}
						{#if !field.optional}
							<span class="text-red-400">*</span>
						{/if}
					</label>
					<input
						id={field.id}
						type={isSecret ? 'password' : 'text'}
						value={fieldValues[field.id] ?? ''}
						oninput={(e) => {
							const target = e.target as HTMLInputElement;
							handleFieldValueChange(field.id, target.value);
							formField.handleChange(target.value);
						}}
						onblur={() => formField.handleBlur()}
						placeholder={field.placeholder ?? ''}
						class="input-field text-primary w-full rounded-md px-3 py-2 text-sm"
						class:input-field-error={formField.state.meta.errors?.length > 0}
					/>
				{/snippet}
			</form.Field>
		{/if}

		{#if field.help_text}
			<p class="text-muted text-xs">{field.help_text}</p>
		{/if}
	</div>
{/snippet}

<style>
	.password-field {
		-webkit-text-security: disc;
	}
</style>

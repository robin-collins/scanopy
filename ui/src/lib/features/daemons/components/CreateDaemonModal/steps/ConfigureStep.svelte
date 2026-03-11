<script lang="ts">
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import type { FormValue } from '$lib/shared/components/forms/validators';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import DocsHint from '$lib/shared/components/feedback/DocsHint.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import SelectNetwork from '$lib/features/networks/components/SelectNetwork.svelte';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import RadioGroup from '$lib/shared/components/forms/input/RadioGroup.svelte';
	import {
		SimpleOptionDisplay,
		type SimpleOption
	} from '$lib/shared/components/forms/selection/display/SimpleOptionDisplay';
	import { ArrowUpCircle } from 'lucide-svelte';
	import { openModal } from '$lib/shared/stores/modal-registry';
	import { fieldDefs } from '../../../config';
	import {
		common_apiKey,
		common_name,
		common_port,
		daemons_config_daemonUrl,
		daemons_config_daemonUrlHelpNoPort,
		daemons_config_mode,
		daemons_config_namePlaceholder,
		daemons_config_portHelpServerPoll,
		daemons_generateNewKey,
		daemons_generateNewKeyHelp,
		daemons_networkCannotChange,
		daemons_pasteApiKey,
		daemons_portForwardingHint,
		daemons_docsPollingMode,
		daemons_docsPollingModeLinkText,
		daemons_httpDaemonUrlWarning,
		daemons_useExistingKey,
		daemons_useExistingKeyHelp,
		daemons_useKey
	} from '$lib/paraglide/messages';

	interface Props {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any };
		formValues: Record<string, string | number | boolean>;
		selectedNetworkId: string;
		onNetworkChange: (id: string) => void;
		onNameInput?: () => void;
		hasDaemonPoll: boolean;
		keySet: boolean;
		isFirstDaemon?: boolean;
		onUseExistingKey?: () => void;
	}

	let {
		form,
		formValues,
		selectedNetworkId,
		onNetworkChange,
		onNameInput,
		hasDaemonPoll,
		keySet,
		isFirstDaemon = false,
		onUseExistingKey
	}: Props = $props();

	// Get validators for a field
	function getValidators(fieldId: string) {
		const def = fieldDefs.find((d) => d.id === fieldId);
		if (!def?.validators || def.validators.length === 0) return {};

		return {
			onBlur: ({ value }: { value: FormValue }) => {
				for (const validator of def.validators!) {
					const error = validator(value);
					if (error) return error;
				}
				return undefined;
			}
		};
	}

	let nameDef = fieldDefs.find((d) => d.id === 'name')!;
	let modeDef = fieldDefs.find((d) => d.id === 'mode')!;
	let daemonUrlDef = fieldDefs.find((d) => d.id === 'daemonUrl')!;
	let daemonPortDef = fieldDefs.find((d) => d.id === 'daemonPort')!;

	let isServerPoll = $derived(formValues.mode === 'server_poll');
	let daemonUrl = $derived(String(formValues.daemonUrl ?? ''));
	let showHttpWarning = $derived.by(() => {
		try {
			const parsed = new URL(daemonUrl);
			if (parsed.protocol !== 'http:') return false;
			const host = parsed.hostname;
			if (host === 'localhost' || host === '127.0.0.1' || host === '::1') return false;
			// Suppress while user is still typing a localhost address
			if ('localhost'.startsWith(host) || '127.0.0.1'.startsWith(host)) return false;
			return true;
		} catch {
			// URL is incomplete/invalid — don't show warning yet
			return false;
		}
	});
</script>

<div class="space-y-4">
	<SelectNetwork
		{selectedNetworkId}
		onNetworkChange={(id) => onNetworkChange(id)}
		disabled={keySet}
		disabledReason={daemons_networkCannotChange()}
	/>

	<!-- Name -->
	<div oninput={() => onNameInput?.()}>
		<form.Field name={nameDef.id} validators={getValidators(nameDef.id)}>
			{#snippet children(field: AnyFieldApi)}
				<TextInput
					label={common_name()}
					{field}
					id={nameDef.id}
					placeholder={daemons_config_namePlaceholder()}
					required={true}
				/>
			{/snippet}
		</form.Field>
	</div>

	<!-- Mode -->
	<form.Field name={modeDef.id}>
		{#snippet children(field: AnyFieldApi)}
			<RichSelect
				label={daemons_config_mode()}
				selectedValue={String(field.state.value ?? '')}
				disabled={keySet}
				options={(modeDef.options ?? []).map((opt): SimpleOption => {
					const needsUpgrade = opt.value === 'daemon_poll' && !hasDaemonPoll;
					return {
						value: opt.value,
						label: opt.label(),
						description:
							opt.value === 'daemon_poll'
								? 'Daemon connects to server; works behind NAT/firewall without opening ports'
								: 'Server connects to daemon; requires providing Daemon URL',
						disabled: needsUpgrade,
						tags: needsUpgrade ? [{ label: 'Upgrade', color: 'Yellow', icon: ArrowUpCircle }] : []
					};
				})}
				onSelect={(value) => field.handleChange(value)}
				onDisabledClick={() => openModal('billing-plan')}
				displayComponent={SimpleOptionDisplay}
			/>
		{/snippet}
	</form.Field>

	<DocsHint
		text={daemons_docsPollingMode()}
		href="https://scanopy.net/docs/setting-up-daemons/planning-daemon-deployment/#choosing-a-polling-mode"
		linkText={daemons_docsPollingModeLinkText()}
	/>

	<!-- Server Poll: URL + Port side-by-side with port forwarding hint -->
	{#if isServerPoll}
		<div class="grid grid-cols-[1fr_auto] gap-4">
			<form.Field name={daemonUrlDef.id} validators={getValidators(daemonUrlDef.id)}>
				{#snippet children(field: AnyFieldApi)}
					<TextInput
						label={daemons_config_daemonUrl()}
						{field}
						id={daemonUrlDef.id}
						placeholder={String(
							typeof daemonUrlDef.placeholder === 'function'
								? daemonUrlDef.placeholder()
								: (daemonUrlDef.placeholder ?? '')
						)}
						required={true}
						helpText={daemons_config_daemonUrlHelpNoPort()}
					/>
				{/snippet}
			</form.Field>

			<div class="w-48">
				<form.Field name={daemonPortDef.id} validators={getValidators(daemonPortDef.id)}>
					{#snippet children(field: AnyFieldApi)}
						<TextInput
							label={common_port()}
							{field}
							id={daemonPortDef.id}
							type="number"
							placeholder={String(daemonPortDef.placeholder ?? '')}
							helpText={daemons_config_portHelpServerPoll()}
						/>
					{/snippet}
				</form.Field>
			</div>
		</div>

		{#if showHttpWarning}
			<InlineWarning title="" body={daemons_httpDaemonUrlWarning()} />
		{/if}

		<InlineInfo title="" body={daemons_portForwardingHint()} />
	{/if}

	<!-- API key info: auto-generated when no key source choice is shown -->
	{#if isFirstDaemon || isServerPoll}
		<InlineInfo
			title=""
			body="An API key will be automatically generated for this daemon when you proceed to the next step."
			dismissableKey="daemon-auto-key-hint"
		/>
	{/if}

	<!-- Inline API key source for DaemonPoll (subsequent daemons only) -->
	{#if !isFirstDaemon && !isServerPoll}
		<div class="border-primary/10 space-y-3 border-t pt-4">
			<form.Field name="keySource">
				{#snippet children(field: AnyFieldApi)}
					<RadioGroup
						label={common_apiKey()}
						id="key-source"
						{field}
						options={[
							{
								value: 'generate',
								label: daemons_generateNewKey(),
								helpText: daemons_generateNewKeyHelp()
							},
							{
								value: 'existing',
								label: daemons_useExistingKey(),
								helpText: daemons_useExistingKeyHelp()
							}
						]}
						disabled={keySet}
					/>
				{/snippet}
			</form.Field>

			{#if formValues.keySource === 'existing'}
				<form.Field name="existingKeyInput">
					{#snippet children(field: AnyFieldApi)}
						<div class="flex items-center gap-2">
							<div class="flex-1">
								<TextInput
									label=""
									{field}
									id="existing-key-input"
									placeholder={daemons_pasteApiKey()}
									disabled={keySet}
								/>
							</div>
							<button
								class="btn-primary flex-shrink-0"
								disabled={keySet || !String(formValues.existingKeyInput ?? '').trim()}
								type="button"
								onclick={() => onUseExistingKey?.()}
							>
								<span>{daemons_useKey()}</span>
							</button>
						</div>
					{/snippet}
				</form.Field>
			{/if}
		</div>
	{/if}
</div>

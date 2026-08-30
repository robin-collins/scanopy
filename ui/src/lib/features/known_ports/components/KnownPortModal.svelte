<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import TextArea from '$lib/shared/components/forms/input/TextArea.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { max, port as validPort, required } from '$lib/shared/components/forms/validators';
	import { createDefaultKnownPort, type KnownPort, type KnownPortInput } from '../types';
	import {
		common_builtin,
		common_cancel,
		common_create,
		common_custom,
		common_delete,
		common_deleting,
		common_description,
		common_editName,
		common_name,
		common_portNumber,
		common_protocol,
		common_save,
		common_saving,
		knownPorts_createTitle,
		knownPorts_portNumberHelp,
		knownPorts_readOnlyHelp,
		knownPorts_viewTitle
	} from '$lib/paraglide/messages';

	let {
		isOpen = false,
		port = null,
		readOnly = false,
		onCreate,
		onUpdate,
		onDelete,
		onClose
	}: {
		isOpen?: boolean;
		port?: KnownPort | null;
		readOnly?: boolean;
		onCreate: (input: KnownPortInput) => Promise<void>;
		onUpdate: (id: string, input: KnownPortInput) => Promise<void>;
		onDelete: (port: KnownPort) => Promise<void>;
		onClose: () => void;
	} = $props();

	let saving = $state(false);
	let deleting = $state(false);
	let isBuiltin = $derived(port?.source === 'BuiltIn');
	let isProtected = $derived(isBuiltin || readOnly);
	let title = $derived(
		port
			? isProtected
				? knownPorts_viewTitle({ name: port.name })
				: common_editName({ name: port.name })
			: knownPorts_createTitle()
	);

	const form = createForm(() => ({
		defaultValues: createDefaultKnownPort(),
		onSubmit: async ({ value }) => {
			if (isProtected) return;
			const input: KnownPortInput = {
				name: value.name.trim(),
				description: value.description?.trim() || null,
				port_number: Number(value.port_number),
				transport_protocol: value.transport_protocol
			};
			saving = true;
			try {
				if (port) await onUpdate(port.id, input);
				else await onCreate(input);
			} finally {
				saving = false;
			}
		}
	}));

	function handleOpen() {
		form.reset(
			port
				? {
						name: port.name,
						description: port.description,
						port_number: port.port_number,
						transport_protocol: port.transport_protocol
					}
				: createDefaultKnownPort()
		);
	}

	async function handleDelete() {
		if (!port || isBuiltin) return;
		deleting = true;
		try {
			await onDelete(port);
		} finally {
			deleting = false;
		}
	}
</script>

<GenericModal {isOpen} {title} size="lg" {onClose} onOpen={handleOpen} showCloseButton={true}>
	<form
		class="flex min-h-0 flex-1 flex-col"
		onsubmit={(event) => {
			event.preventDefault();
			event.stopPropagation();
			submitForm(form);
		}}
	>
		<div class="min-h-0 flex-1 space-y-5 overflow-auto p-6">
			{#if port}
				<div
					class="rounded-lg border border-gray-200 bg-gray-50 p-3 text-sm dark:border-gray-700 dark:bg-gray-800"
				>
					<div class="font-medium">{isBuiltin ? common_builtin() : common_custom()}</div>
					{#if isBuiltin}<div class="text-secondary mt-1">{knownPorts_readOnlyHelp()}</div>{/if}
				</div>
			{/if}

			<form.Field
				name="name"
				validators={{ onBlur: ({ value }) => required(value) || max(100)(value) }}
			>
				{#snippet children(field)}
					<TextInput
						label={common_name()}
						id="known-port-name"
						{field}
						required
						disabled={isProtected}
					/>
				{/snippet}
			</form.Field>

			<form.Field name="description" validators={{ onBlur: ({ value }) => max(500)(value || '') }}>
				{#snippet children(field)}
					<TextArea
						label={common_description()}
						id="known-port-description"
						{field}
						disabled={isProtected}
					/>
				{/snippet}
			</form.Field>

			<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
				<form.Field name="port_number" validators={{ onBlur: ({ value }) => validPort(value) }}>
					{#snippet children(field)}
						<TextInput
							label={common_portNumber()}
							id="known-port-number"
							{field}
							type="number"
							required
							helpText={knownPorts_portNumberHelp()}
							disabled={isProtected}
						/>
					{/snippet}
				</form.Field>

				<form.Field name="transport_protocol">
					{#snippet children(field)}
						<SelectInput
							label={common_protocol()}
							id="known-port-protocol"
							{field}
							options={[
								{ value: 'Tcp', label: 'TCP' },
								{ value: 'Udp', label: 'UDP' }
							]}
							disabled={isProtected}
						/>
					{/snippet}
				</form.Field>
			</div>
		</div>

		<div class="modal-footer">
			<div class="flex items-center justify-between">
				<div>
					{#if port && !isProtected}
						<button
							type="button"
							class="btn-danger"
							disabled={saving || deleting}
							onclick={handleDelete}
						>
							{deleting ? common_deleting() : common_delete()}
						</button>
					{/if}
				</div>
				<div class="flex gap-3">
					<button
						type="button"
						class="btn-secondary"
						disabled={saving || deleting}
						onclick={onClose}
					>
						{common_cancel()}
					</button>
					{#if !isProtected}
						<button type="submit" class="btn-primary" disabled={saving || deleting}>
							{saving ? common_saving() : port ? common_save() : common_create()}
						</button>
					{/if}
				</div>
			</div>
		</div>
	</form>
</GenericModal>

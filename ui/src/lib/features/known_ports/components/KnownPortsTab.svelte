<script lang="ts">
	import { Eye, Lock, Pencil, Plus } from 'lucide-svelte';
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import type { TabProps } from '$lib/shared/types';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { permissions } from '$lib/shared/stores/metadata';
	import {
		useCreateKnownPortMutation,
		useDeleteKnownPortMutation,
		useKnownPortsQuery,
		useUpdateKnownPortMutation
	} from '../queries';
	import type { KnownPort, KnownPortInput } from '../types';
	import KnownPortModal from './KnownPortModal.svelte';
	import {
		common_builtin,
		common_confirmDeleteName,
		common_create,
		common_custom,
		common_description,
		common_name,
		common_noEntityYet,
		common_portNumber,
		common_protocol,
		common_source,
		knownPorts_subtitle,
		knownPorts_title
	} from '$lib/paraglide/messages';

	let { isReadOnly = false }: TabProps = $props();
	const currentUserQuery = useCurrentUserQuery();
	const portsQuery = useKnownPortsQuery();
	const createMutation = useCreateKnownPortMutation();
	const updateMutation = useUpdateKnownPortMutation();
	const deleteMutation = useDeleteKnownPortMutation();

	let selectedPort = $state<KnownPort | null>(null);
	let modalOpen = $state(false);
	let ports = $derived(
		[...(portsQuery.data ?? [])].sort(
			(a, b) =>
				a.port_number - b.port_number ||
				a.transport_protocol.localeCompare(b.transport_protocol) ||
				a.name.localeCompare(b.name)
		)
	);
	let canManage = $derived(
		!isReadOnly &&
			!!currentUserQuery.data &&
			permissions.getMetadata(currentUserQuery.data.permissions).manage_org_entities
	);

	function openCreate() {
		selectedPort = null;
		modalOpen = true;
	}

	function openPort(port: KnownPort) {
		selectedPort = port;
		modalOpen = true;
	}

	function closeModal() {
		modalOpen = false;
		selectedPort = null;
	}

	async function createPort(input: KnownPortInput) {
		await createMutation.mutateAsync(input);
		closeModal();
	}

	async function updatePort(id: string, input: KnownPortInput) {
		await updateMutation.mutateAsync({ id, input });
		closeModal();
	}

	async function deletePort(port: KnownPort) {
		if (!confirm(common_confirmDeleteName({ name: port.name }))) return;
		await deleteMutation.mutateAsync(port.id);
		closeModal();
	}
</script>

<div class="space-y-6">
	<TabHeader title={knownPorts_title()} subtitle={knownPorts_subtitle()}>
		<svelte:fragment slot="actions">
			{#if canManage}
				<button class="btn-primary flex items-center gap-2" onclick={openCreate}>
					<Plus class="h-5 w-5" />{common_create()}
				</button>
			{/if}
		</svelte:fragment>
	</TabHeader>

	{#if portsQuery.isLoading}
		<Loading />
	{:else if ports.length === 0}
		<EmptyState
			title={common_noEntityYet({ entity: knownPorts_title() })}
			onClick={canManage ? openCreate : undefined}
			cta={canManage ? common_create() : ''}
		/>
	{:else}
		<div class="card overflow-x-auto p-0">
			<table class="w-full min-w-[760px] text-left text-sm">
				<thead class="border-b border-gray-200 bg-gray-50 dark:border-gray-700 dark:bg-gray-800">
					<tr>
						<th class="px-4 py-3 font-medium">{common_portNumber()}</th>
						<th class="px-4 py-3 font-medium">{common_protocol()}</th>
						<th class="px-4 py-3 font-medium">{common_name()}</th>
						<th class="px-4 py-3 font-medium">{common_description()}</th>
						<th class="px-4 py-3 font-medium">{common_source()}</th>
						<th class="w-12 px-4 py-3"><span class="sr-only">Open</span></th>
					</tr>
				</thead>
				<tbody class="divide-y divide-gray-200 dark:divide-gray-700">
					{#each ports as port (port.id)}
						<tr class="hover:bg-gray-50 dark:hover:bg-gray-800/60">
							<td class="px-4 py-3 font-mono">{port.port_number}</td>
							<td class="px-4 py-3">{port.transport_protocol.toUpperCase()}</td>
							<td class="text-primary px-4 py-3 font-medium">{port.name}</td>
							<td class="text-secondary max-w-md px-4 py-3">{port.description ?? '—'}</td>
							<td class="px-4 py-3">
								<Tag
									label={port.source === 'BuiltIn' ? common_builtin() : common_custom()}
									icon={port.source === 'BuiltIn' ? Lock : undefined}
									pill
								/>
							</td>
							<td class="px-4 py-3">
								<button
									class="btn-icon"
									onclick={() => openPort(port)}
									aria-label={port.source === 'BuiltIn' ? `View ${port.name}` : `Edit ${port.name}`}
								>
									{#if port.source === 'BuiltIn'}
										<Eye size={16} />
									{:else}
										<Pencil size={16} />
									{/if}
								</button>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</div>

<KnownPortModal
	isOpen={modalOpen}
	port={selectedPort}
	readOnly={!canManage}
	onCreate={createPort}
	onUpdate={updatePort}
	onDelete={deletePort}
	onClose={closeModal}
/>

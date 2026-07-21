<script lang="ts">
	import { PenTool, Plus, ChevronDown, Trash2 } from 'lucide-svelte';
	import {
		useCustomTopologyViewsQuery,
		useCreateCustomTopologyViewMutation,
		useDeleteCustomTopologyViewMutation,
		type CustomTopologyView
	} from '$lib/features/custom-topology-views/queries';
	import { pushError } from '$lib/shared/stores/feedback';
	import {
		topology_customViewDeleteView,
		topology_customViewNamePlaceholder
	} from '$lib/paraglide/messages';

	interface Props {
		networkId: string;
		selectedView: CustomTopologyView | null;
		onSelect: (view: CustomTopologyView | null) => void;
	}

	let { networkId, selectedView, onSelect }: Props = $props();

	const viewsQuery = useCustomTopologyViewsQuery(() => networkId);
	const createMutation = useCreateCustomTopologyViewMutation();
	const deleteMutation = useDeleteCustomTopologyViewMutation();

	let menuOpen = $state(false);
	let creating = $state(false);
	let newViewName = $state('');

	async function handleCreate() {
		if (!newViewName.trim()) return;
		try {
			const view = await createMutation.mutateAsync({ networkId, name: newViewName.trim() });
			newViewName = '';
			creating = false;
			menuOpen = false;
			onSelect(view);
		} catch (e) {
			pushError(e instanceof Error ? e.message : 'Failed to create view');
		}
	}

	async function handleDelete(view: CustomTopologyView, event: MouseEvent) {
		event.stopPropagation();
		try {
			await deleteMutation.mutateAsync({ id: view.id, networkId });
			if (selectedView?.id === view.id) onSelect(null);
		} catch (e) {
			pushError(e instanceof Error ? e.message : 'Failed to delete view');
		}
	}
</script>

<div class="relative">
	<button class="btn-secondary flex items-center gap-1.5" onclick={() => (menuOpen = !menuOpen)}>
		<PenTool class="h-4 w-4 text-pink-500" />
		<span class="max-w-[10rem] truncate text-sm">{selectedView?.name ?? 'Custom Views'}</span>
		<ChevronDown class="h-3.5 w-3.5" />
	</button>

	{#if menuOpen}
		<div class="card-static absolute right-0 top-full z-20 mt-1 w-56 space-y-1 p-2 shadow-lg">
			{#each viewsQuery.data ?? [] as view (view.id)}
				<div
					class="group flex items-center justify-between gap-1 rounded px-2 py-1 hover:bg-gray-100 dark:hover:bg-gray-800"
				>
					<button
						class="flex-1 truncate text-left text-sm"
						class:font-semibold={selectedView?.id === view.id}
						onclick={() => {
							onSelect(view);
							menuOpen = false;
						}}
					>
						{view.name}
					</button>
					<button
						class="hidden text-red-400 group-hover:block"
						title={topology_customViewDeleteView()}
						onclick={(e) => handleDelete(view, e)}
					>
						<Trash2 class="h-3.5 w-3.5" />
					</button>
				</div>
			{/each}

			{#if (viewsQuery.data ?? []).length === 0 && !creating}
				<p class="px-2 py-1 text-xs text-gray-400">No custom views yet.</p>
			{/if}

			{#if creating}
				<div class="flex gap-1 px-2">
					<input
						class="input-field flex-1 text-xs"
						placeholder={topology_customViewNamePlaceholder()}
						bind:value={newViewName}
						onkeydown={(e) => e.key === 'Enter' && handleCreate()}
					/>
					<button class="btn-secondary text-xs" onclick={handleCreate}>Add</button>
				</div>
			{:else}
				<button
					class="flex w-full items-center gap-1.5 rounded px-2 py-1 text-sm text-blue-600 hover:bg-gray-100 dark:hover:bg-gray-800"
					onclick={() => (creating = true)}
				>
					<Plus class="h-3.5 w-3.5" /> New view
				</button>
			{/if}
		</div>
	{/if}
</div>

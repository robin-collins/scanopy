<script lang="ts">
	import { Search, Type, Square, Plus, Upload } from 'lucide-svelte';
	import type { Host } from '$lib/features/hosts/types/base';
	import type { Service } from '$lib/features/services/types/base';
	import type { LibraryObject } from '$lib/features/custom-topology-views/queries';
	import {
		useCreateLibraryObjectMutation,
		useUploadLibraryObjectImageMutation,
		libraryObjectImageUrl
	} from '$lib/features/custom-topology-views/queries';
	import { createIconComponent, createColorHelper } from '$lib/shared/utils/styling';
	import {
		topology_customViewSearchPlaceholder,
		topology_customViewAddObject,
		topology_customViewObjectNamePlaceholder,
		topology_customViewUploadImage
	} from '$lib/paraglide/messages';

	interface Props {
		hosts: Host[];
		services: Service[];
		libraryObjects: LibraryObject[];
	}

	let { hosts, services, libraryObjects }: Props = $props();

	let search = $state('');
	let showAddForm = $state(false);
	let newObjectName = $state('');

	const createLibraryObject = useCreateLibraryObjectMutation();
	const uploadLibraryObjectImage = useUploadLibraryObjectImageMutation();

	let filteredHosts = $derived(
		hosts.filter((h) => h.name.toLowerCase().includes(search.toLowerCase()))
	);
	let filteredServices = $derived(
		services.filter((s) => s.name.toLowerCase().includes(search.toLowerCase()))
	);
	let filteredLibraryObjects = $derived(
		libraryObjects.filter((o) => o.name.toLowerCase().includes(search.toLowerCase()))
	);

	function dragPayload(payload: unknown) {
		return (event: DragEvent) => {
			event.dataTransfer?.setData('application/x-scanopy-palette-item', JSON.stringify(payload));
			if (event.dataTransfer) event.dataTransfer.effectAllowed = 'copy';
		};
	}

	async function handleAddObject() {
		if (!newObjectName.trim()) return;
		await createLibraryObject.mutateAsync({
			name: newObjectName.trim(),
			icon: null,
			color: 'Gray'
		});
		newObjectName = '';
		showAddForm = false;
	}

	async function handleUploadForObject(objectId: string, event: Event) {
		const file = (event.target as HTMLInputElement).files?.[0];
		if (file) await uploadLibraryObjectImage.mutateAsync({ objectId, file });
	}
</script>

<div
	class="flex w-64 flex-col gap-3 overflow-y-auto border-r border-gray-200 bg-white p-3 dark:border-gray-700 dark:bg-gray-900"
>
	<div class="relative">
		<Search class="absolute left-2 top-2 h-4 w-4 text-gray-400" />
		<input
			class="input-field w-full pl-7 text-sm"
			placeholder={topology_customViewSearchPlaceholder()}
			bind:value={search}
		/>
	</div>

	<div>
		<h4 class="mb-1 text-xs font-semibold uppercase tracking-wide text-gray-500">Annotations</h4>
		<div class="grid grid-cols-2 gap-2">
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class="flex cursor-grab flex-col items-center gap-1 rounded border border-gray-200 p-2 text-xs dark:border-gray-700"
				draggable="true"
				ondragstart={dragPayload({ kind: 'text' })}
			>
				<Type class="h-5 w-5" />
				Text
			</div>
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class="flex cursor-grab flex-col items-center gap-1 rounded border border-gray-200 p-2 text-xs dark:border-gray-700"
				draggable="true"
				ondragstart={dragPayload({ kind: 'group' })}
			>
				<Square class="h-5 w-5" />
				Group frame
			</div>
		</div>
	</div>

	{#if filteredHosts.length > 0}
		<div>
			<h4 class="mb-1 text-xs font-semibold uppercase tracking-wide text-gray-500">Hosts</h4>
			<div class="space-y-1">
				{#each filteredHosts as host (host.id)}
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<div
						class="cursor-grab truncate rounded border border-gray-200 px-2 py-1 text-xs dark:border-gray-700"
						draggable="true"
						ondragstart={dragPayload({
							kind: 'entity',
							entityType: 'Host',
							entityId: host.id,
							label: host.name
						})}
						title={host.name}
					>
						{host.name}
					</div>
				{/each}
			</div>
		</div>
	{/if}

	{#if filteredServices.length > 0}
		<div>
			<h4 class="mb-1 text-xs font-semibold uppercase tracking-wide text-gray-500">Services</h4>
			<div class="space-y-1">
				{#each filteredServices as service (service.id)}
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<div
						class="cursor-grab truncate rounded border border-gray-200 px-2 py-1 text-xs dark:border-gray-700"
						draggable="true"
						ondragstart={dragPayload({
							kind: 'entity',
							entityType: 'Service',
							entityId: service.id,
							label: service.name
						})}
						title={service.name}
					>
						{service.name}
					</div>
				{/each}
			</div>
		</div>
	{/if}

	<div>
		<div class="mb-1 flex items-center justify-between">
			<h4 class="text-xs font-semibold uppercase tracking-wide text-gray-500">Common objects</h4>
			<button
				class="btn-icon"
				title={topology_customViewAddObject()}
				onclick={() => (showAddForm = !showAddForm)}
			>
				<Plus class="h-3.5 w-3.5" />
			</button>
		</div>

		{#if showAddForm}
			<div class="mb-2 flex gap-1">
				<input
					class="input-field flex-1 text-xs"
					placeholder={topology_customViewObjectNamePlaceholder()}
					bind:value={newObjectName}
					onkeydown={(e) => e.key === 'Enter' && handleAddObject()}
				/>
				<button class="btn-secondary text-xs" onclick={handleAddObject}>Add</button>
			</div>
		{/if}

		<div class="grid grid-cols-3 gap-2">
			{#each filteredLibraryObjects as obj (obj.id)}
				{@const IconComponent = obj.icon ? createIconComponent(obj.icon) : null}
				{@const colorStyle = createColorHelper(obj.color ?? null)}
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class="group relative flex cursor-grab flex-col items-center gap-1 rounded border border-gray-200 p-2 dark:border-gray-700"
					draggable="true"
					ondragstart={dragPayload({ kind: 'library', libraryObjectId: obj.id, label: obj.name })}
					title={obj.name}
				>
					{#if obj.storage_path}
						<img src={libraryObjectImageUrl(obj.id)} alt="" class="h-6 w-6 rounded object-cover" />
					{:else if IconComponent}
						<IconComponent class="h-6 w-6 {colorStyle.icon}" />
					{/if}
					<span class="w-full truncate text-center text-[10px]">{obj.name}</span>
					{#if obj.organization_id}
						<label
							class="absolute right-0.5 top-0.5 hidden cursor-pointer group-hover:block"
							title={topology_customViewUploadImage()}
						>
							<Upload class="h-3 w-3 text-gray-400" />
							<input
								type="file"
								accept="image/*"
								class="hidden"
								onchange={(e) => handleUploadForObject(obj.id, e)}
							/>
						</label>
					{/if}
				</div>
			{/each}
		</div>
	</div>
</div>

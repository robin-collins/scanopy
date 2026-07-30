<script lang="ts">
	import {
		Search,
		Type,
		Square,
		Plus,
		Upload,
		Server,
		GripVertical,
		ChevronDown,
		ChevronRight,
		Layers
	} from 'lucide-svelte';
	import type { Host } from '$lib/features/hosts/types/base';
	import type { Service } from '$lib/features/services/types/base';
	import type { LibraryObject } from '$lib/features/custom-topology-views/queries';
	import {
		useCreateLibraryObjectMutation,
		useUploadLibraryObjectImageMutation,
		libraryObjectImageUrl
	} from '$lib/features/custom-topology-views/queries';
	import { createIconComponent, createColorHelper } from '$lib/shared/utils/styling';
	import { serviceDefinitions } from '$lib/shared/stores/metadata';
	import { filterPaletteHosts, getHostServices } from './custom-view-model';
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
	let selectedHostId = $state<string | null>(null);
	let servicesExpanded = $state(false);

	const createLibraryObject = useCreateLibraryObjectMutation();
	const uploadLibraryObjectImage = useUploadLibraryObjectImageMutation();

	let filteredHosts = $derived(filterPaletteHosts(hosts, services, search));
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

	function toggleHost(hostId: string) {
		selectedHostId = selectedHostId === hostId ? null : hostId;
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
	<div class="sticky top-0 z-10 bg-white dark:bg-gray-900">
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
					{@const hostServices = getHostServices(services, host.id)}
					<div
						class="overflow-hidden rounded border border-gray-200 bg-white transition-colors dark:border-gray-700 dark:bg-gray-900"
						class:ring-2={selectedHostId === host.id}
						class:ring-blue-500={selectedHostId === host.id}
					>
						<button
							type="button"
							class="flex w-full cursor-grab items-center gap-1.5 px-1.5 py-1.5 text-left"
							draggable="true"
							ondragstart={dragPayload({
								kind: 'entity',
								entityType: 'Host',
								entityId: host.id,
								label: host.name
							})}
							onclick={() => toggleHost(host.id)}
							aria-expanded={selectedHostId === host.id}
							title={`${host.name} — click to preview, drag to add`}
						>
							<GripVertical class="h-3.5 w-3.5 flex-shrink-0 text-gray-400" />
							<Server class="h-4 w-4 flex-shrink-0 text-blue-500" />
							<span class="min-w-0 flex-1">
								<span class="text-primary block truncate text-xs font-medium">{host.name}</span>
								<span class="text-tertiary block truncate text-[10px]">
									{host.hostname || `${hostServices.length} services`}
								</span>
							</span>
							{#if selectedHostId === host.id}
								<ChevronDown class="h-3.5 w-3.5 flex-shrink-0 text-gray-400" />
							{:else}
								<ChevronRight class="h-3.5 w-3.5 flex-shrink-0 text-gray-400" />
							{/if}
						</button>

						{#if selectedHostId === host.id}
							<div class="border-t border-gray-200 p-2 dark:border-gray-700">
								<div
									class="overflow-hidden rounded-lg border border-gray-300 bg-gray-50 shadow-sm dark:border-gray-600 dark:bg-gray-800"
								>
									<div class="border-b border-gray-200 px-2 py-1.5 dark:border-gray-700">
										<span class="text-tertiary block truncate text-[10px] font-medium">
											{host.hostname || host.manufacturer || 'Host'}
										</span>
										<div class="flex items-center gap-1.5">
											<Server class="h-4 w-4 flex-shrink-0 text-blue-500" />
											<span class="text-primary truncate text-xs font-semibold">{host.name}</span>
										</div>
									</div>
									<div class="space-y-1 px-2 py-2">
										{#each hostServices.slice(0, 6) as service (service.id)}
											{@const ServiceIcon = serviceDefinitions.getIconComponent(
												service.service_definition
											)}
											{@const serviceColor = serviceDefinitions.getColorHelper(
												service.service_definition
											)}
											<div class="flex items-center gap-1.5" title={service.name}>
												<ServiceIcon class="h-3.5 w-3.5 flex-shrink-0 {serviceColor.icon}" />
												<span class="text-secondary truncate text-[11px]">{service.name}</span>
											</div>
										{:else}
											<span class="text-tertiary text-[11px]">No services found</span>
										{/each}
										{#if hostServices.length > 6}
											<span class="text-tertiary block text-[10px]">
												+{hostServices.length - 6} more services
											</span>
										{/if}
									</div>
								</div>
								<p class="text-tertiary mt-1 text-center text-[10px]">
									Stats card preview · drag this host to add it
								</p>
							</div>
						{/if}
					</div>
				{/each}
			</div>
		</div>
	{/if}

	{#if filteredServices.length > 0}
		<div>
			<button
				type="button"
				class="mb-1 flex w-full items-center justify-between text-xs font-semibold uppercase tracking-wide text-gray-500"
				onclick={() => (servicesExpanded = !servicesExpanded)}
				aria-expanded={servicesExpanded || search.trim().length > 0}
			>
				<span class="flex items-center gap-1">
					<Layers class="h-3.5 w-3.5" />
					Services ({filteredServices.length})
				</span>
				{#if servicesExpanded || search.trim().length > 0}
					<ChevronDown class="h-3.5 w-3.5" />
				{:else}
					<ChevronRight class="h-3.5 w-3.5" />
				{/if}
			</button>
			{#if servicesExpanded || search.trim().length > 0}
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
			{/if}
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

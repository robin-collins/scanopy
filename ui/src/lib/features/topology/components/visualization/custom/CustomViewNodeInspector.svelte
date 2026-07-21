<script lang="ts">
	import { Trash2, Upload } from 'lucide-svelte';
	import type {
		CustomViewNode,
		LibraryObject,
		NodeStyle,
		CornerStyle
	} from '$lib/features/custom-topology-views/queries';
	import type { components } from '$lib/api/schema';
	import { createColorHelper } from '$lib/shared/utils/styling';
	import { common_delete } from '$lib/paraglide/messages';

	type Color = components['schemas']['Color'];

	interface Props {
		node: CustomViewNode;
		libraryObjects: LibraryObject[];
		onUpdate: (patch: Partial<CustomViewNode>) => void;
		onUploadImage: (file: File) => void;
		onDelete: () => void;
	}

	let { node, libraryObjects, onUpdate, onUploadImage, onDelete }: Props = $props();

	const STYLES: { value: NodeStyle; label: string }[] = [
		{ value: 'Image', label: 'Image' },
		{ value: 'ImageBordered', label: 'Bordered image' },
		{ value: 'Badge', label: '1-2 letter badge' },
		{ value: 'StatsCard', label: 'Stats card' }
	];

	const CORNER_STYLES: { value: CornerStyle; label: string }[] = [
		{ value: 'Rounded', label: 'Rounded' },
		{ value: 'Square', label: 'Square' }
	];

	const COLORS: Color[] = [
		'Pink',
		'Rose',
		'Red',
		'Amber',
		'Orange',
		'Green',
		'Emerald',
		'Teal',
		'Cyan',
		'Blue',
		'Indigo',
		'Purple',
		'Fuchsia',
		'Violet',
		'Sky',
		'Gray',
		'Lime',
		'Yellow'
	];

	let isObjectKind = $derived(node.kind === 'Entity' || node.kind === 'Library');
	let statsCardAvailable = $derived(node.kind === 'Entity' && node.entity_type === 'Host');
	let libraryObjectName = $derived(
		node.kind === 'Library'
			? (libraryObjects.find((o) => o.id === node.library_object_id)?.name ?? null)
			: null
	);
	let fileInput: HTMLInputElement | undefined = $state();

	function handleFileChange(e: Event) {
		const file = (e.target as HTMLInputElement).files?.[0];
		if (file) onUploadImage(file);
	}
</script>

<div
	class="absolute right-2 top-2 z-10 w-64 space-y-3 rounded-md bg-white p-3 shadow-lg dark:bg-gray-900"
>
	<div class="flex items-center justify-between">
		<span class="text-sm font-semibold">
			{node.kind} node{libraryObjectName ? ` — ${libraryObjectName}` : ''}
		</span>
		<button class="btn-icon text-red-500" title={common_delete()} onclick={onDelete}>
			<Trash2 class="h-4 w-4" />
		</button>
	</div>

	{#if node.kind !== 'Group' && node.kind !== 'Text'}
		<label class="block text-xs font-medium">
			Label
			<input
				class="input-field mt-1 w-full"
				value={node.label ?? ''}
				oninput={(e) => onUpdate({ label: (e.target as HTMLInputElement).value })}
			/>
		</label>
	{/if}

	{#if isObjectKind}
		<div>
			<span class="block text-xs font-medium">Look</span>
			<div class="mt-1 grid grid-cols-1 gap-1">
				{#each STYLES as opt (opt.value)}
					{#if opt.value !== 'StatsCard' || statsCardAvailable}
						<label class="flex items-center gap-2 text-xs">
							<input
								type="radio"
								name="node-style-{node.id}"
								checked={(node.style ?? 'Image') === opt.value}
								onchange={() => onUpdate({ style: opt.value })}
							/>
							{opt.label}
						</label>
					{/if}
				{/each}
			</div>
		</div>

		{#if node.style === 'Badge'}
			<label class="block text-xs font-medium">
				Badge text (max 2 chars)
				<input
					class="input-field mt-1 w-full"
					maxlength={2}
					value={node.badge_text ?? ''}
					oninput={(e) => onUpdate({ badge_text: (e.target as HTMLInputElement).value })}
				/>
			</label>
		{/if}

		<div>
			<span class="block text-xs font-medium">Custom image</span>
			<button
				class="btn-secondary mt-1 flex w-full items-center justify-center gap-2 text-xs"
				onclick={() => fileInput?.click()}
			>
				<Upload class="h-3.5 w-3.5" /> Upload image
			</button>
			<input
				bind:this={fileInput}
				type="file"
				accept="image/*"
				class="hidden"
				onchange={handleFileChange}
			/>
		</div>
	{/if}

	{#if node.kind === 'Group'}
		<div>
			<span class="block text-xs font-medium">Corner style</span>
			<div class="mt-1 flex gap-3">
				{#each CORNER_STYLES as opt (opt.value)}
					<label class="flex items-center gap-1 text-xs">
						<input
							type="radio"
							name="corner-style-{node.id}"
							checked={(node.corner_style ?? 'Rounded') === opt.value}
							onchange={() => onUpdate({ corner_style: opt.value })}
						/>
						{opt.label}
					</label>
				{/each}
			</div>
		</div>
	{/if}

	{#if node.kind === 'Group' || node.kind === 'Entity' || node.kind === 'Library'}
		<div>
			<span class="block text-xs font-medium">Color</span>
			<div class="mt-1 grid grid-cols-6 gap-1">
				{#each COLORS as color (color)}
					<button
						class="h-5 w-5 rounded-full border"
						class:ring-2={node.color === color}
						style:background-color={createColorHelper(color).rgb}
						title={color}
						onclick={() => onUpdate({ color })}
					></button>
				{/each}
			</div>
		</div>
	{/if}
</div>

<script lang="ts">
	import { PenTool, Blocks, Trash2, X, Settings, ChevronUp } from 'lucide-svelte';
	import { createColorHelper } from '$lib/shared/utils/styling';
	import type { components } from '$lib/api/schema';
	import type { CustomTopologyView } from '$lib/features/custom-topology-views/queries';
	import FontPicker from './FontPicker.svelte';
	import {
		common_close,
		topology_customViewCanvasSettings,
		topology_customViewDefaultFont,
		topology_customViewDeleteView,
		topology_customViewTogglePalette
	} from '$lib/paraglide/messages';

	type Color = components['schemas']['Color'];

	interface Props {
		view: CustomTopologyView;
		onUpdate: (patch: Partial<CustomTopologyView>) => void;
		onTogglePalette: () => void;
		onDelete: () => void;
		onClose: () => void;
	}

	let { view, onUpdate, onTogglePalette, onDelete, onClose }: Props = $props();

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

	let expanded = $state(false);
	let nameDraft = $state('');
	let descriptionDraft = $state('');
	let draftViewId = $state<string | null>(null);

	$effect(() => {
		if (view.id !== draftViewId) {
			draftViewId = view.id;
			nameDraft = view.name;
			descriptionDraft = view.description ?? '';
		}
	});

	function commitName() {
		if (nameDraft.trim() && nameDraft !== view.name) onUpdate({ name: nameDraft.trim() });
	}

	function commitDescription() {
		if (descriptionDraft !== (view.description ?? '')) onUpdate({ description: descriptionDraft });
	}

	function handleGridSizeChange(event: Event) {
		const value = Number((event.target as HTMLInputElement).value);
		if (Number.isInteger(value) && value >= 5 && value <= 200) onUpdate({ grid_size: value });
	}

	function handleFontSizeChange(event: Event) {
		const raw = (event.target as HTMLInputElement).value;
		if (raw === '') {
			onUpdate({ default_font_size: null });
			return;
		}
		const value = Number(raw);
		if (Number.isInteger(value) && value >= 10 && value <= 72)
			onUpdate({ default_font_size: value });
	}

	function stopCanvasInteraction(event: Event) {
		event.stopPropagation();
	}
</script>

<div
	class="absolute left-2 top-2 z-10 max-w-xs rounded-md bg-white/95 shadow dark:bg-gray-900/95"
	role="dialog"
	aria-label={topology_customViewCanvasSettings()}
	tabindex="-1"
	onpointerdown={stopCanvasInteraction}
	onkeydown={stopCanvasInteraction}
>
	<div class="flex items-center gap-2 px-3 py-1.5">
		<PenTool class="h-4 w-4 flex-shrink-0 text-pink-500" />
		<span class="truncate text-sm font-medium">{view.name}</span>
		<button
			class="btn-icon ml-auto"
			title={topology_customViewCanvasSettings()}
			onclick={() => (expanded = !expanded)}
		>
			{#if expanded}<ChevronUp class="h-4 w-4" />{:else}<Settings class="h-4 w-4" />{/if}
		</button>
		<button class="btn-icon" title={topology_customViewTogglePalette()} onclick={onTogglePalette}>
			<Blocks class="h-4 w-4" />
		</button>
		<button
			class="btn-icon text-red-500"
			title={topology_customViewDeleteView()}
			onclick={onDelete}
		>
			<Trash2 class="h-4 w-4" />
		</button>
		<button class="btn-icon" title={common_close()} onclick={onClose}>
			<X class="h-4 w-4" />
		</button>
	</div>

	{#if expanded}
		<div
			class="max-h-[70vh] space-y-3 overflow-y-auto border-t border-gray-200 p-3 dark:border-gray-700"
		>
			<label class="block text-xs font-medium">
				Name
				<input
					class="input-field mt-1 w-full"
					value={nameDraft}
					oninput={(e) => (nameDraft = (e.target as HTMLInputElement).value)}
					onblur={commitName}
					onkeydown={(e) => e.key === 'Enter' && (e.target as HTMLInputElement).blur()}
				/>
			</label>
			<label class="block text-xs font-medium">
				Description
				<textarea
					class="input-field mt-1 w-full"
					rows="2"
					value={descriptionDraft}
					oninput={(e) => (descriptionDraft = (e.target as HTMLTextAreaElement).value)}
					onblur={commitDescription}
				></textarea>
			</label>

			<div>
				<span class="block text-xs font-medium">Background colour</span>
				<div class="mt-1 grid grid-cols-6 gap-1">
					<button
						class="col-span-2 flex h-5 items-center justify-center rounded border text-[10px]"
						class:ring-2={!view.background_color}
						onclick={() => onUpdate({ background_color: null })}
					>
						None
					</button>
					{#each COLORS as color (color)}
						<button
							class="h-5 w-5 rounded-full border"
							class:ring-2={view.background_color === color}
							style:background-color={createColorHelper(color).rgb}
							title={color}
							onclick={() => onUpdate({ background_color: color })}
						></button>
					{/each}
				</div>
			</div>

			<div class="flex items-center gap-3">
				<label class="flex items-center gap-1.5 text-xs">
					<input
						type="checkbox"
						checked={view.show_grid ?? true}
						onchange={(e) => onUpdate({ show_grid: (e.target as HTMLInputElement).checked })}
					/> Show grid
				</label>
				<label class="flex items-center gap-1.5 text-xs">
					<input
						type="checkbox"
						checked={view.snap_to_grid ?? true}
						onchange={(e) => onUpdate({ snap_to_grid: (e.target as HTMLInputElement).checked })}
					/> Snap to grid
				</label>
			</div>
			<label class="block text-xs font-medium">
				Grid size ({view.grid_size ?? 20}px)
				<input
					class="input-field mt-1 w-full"
					type="number"
					min="5"
					max="200"
					step="1"
					value={view.grid_size ?? 20}
					onchange={handleGridSizeChange}
				/>
			</label>

			<div class="border-t border-gray-200 pt-3 dark:border-gray-700">
				<span class="mb-1 block text-xs font-semibold uppercase tracking-wide text-gray-500">
					Defaults for new objects
				</span>
				<FontPicker
					label={topology_customViewDefaultFont()}
					value={view.default_font_family ?? null}
					onSelect={(fontId) => onUpdate({ default_font_family: fontId })}
				/>
				<label class="mt-2 block text-xs font-medium">
					Default font size
					<input
						class="input-field mt-1 w-full"
						type="number"
						min="10"
						max="72"
						step="1"
						placeholder="16"
						value={view.default_font_size ?? ''}
						onchange={handleFontSizeChange}
					/>
				</label>
				<div class="mt-2">
					<span class="block text-xs font-medium">Default object colour</span>
					<div class="mt-1 grid grid-cols-6 gap-1">
						{#each COLORS as color (color)}
							<button
								class="h-5 w-5 rounded-full border"
								class:ring-2={view.default_primary_color === color}
								style:background-color={createColorHelper(color).rgb}
								title={color}
								onclick={() => onUpdate({ default_primary_color: color })}
							></button>
						{/each}
					</div>
				</div>
				<div class="mt-2">
					<span class="block text-xs font-medium">Default connector colour</span>
					<div class="mt-1 grid grid-cols-6 gap-1">
						{#each COLORS as color (color)}
							<button
								class="h-5 w-5 rounded-full border"
								class:ring-2={view.default_connector_color === color}
								style:background-color={createColorHelper(color).rgb}
								title={color}
								onclick={() => onUpdate({ default_connector_color: color })}
							></button>
						{/each}
					</div>
				</div>
			</div>
		</div>
	{/if}
</div>

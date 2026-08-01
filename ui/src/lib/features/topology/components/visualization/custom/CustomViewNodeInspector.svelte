<script lang="ts">
	import { Trash2, Upload } from 'lucide-svelte';
	import type {
		CustomViewNode,
		LibraryObject,
		NodeStyle,
		CornerStyle,
		TextFont,
		FontStyle,
		BorderStyle
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

	const TEXT_FONTS: { value: TextFont; label: string }[] = [
		{ value: 'Sans', label: 'Sans serif' },
		{ value: 'Serif', label: 'Serif' },
		{ value: 'Monospace', label: 'Monospace' }
	];
	const FONT_STYLES: FontStyle[] = ['Normal', 'Bold', 'Italic', 'BoldItalic'];
	const BORDER_STYLES: BorderStyle[] = ['None', 'Solid', 'Dashed', 'Dotted', 'Double'];

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
	let labelDraft = $state('');
	let badgeDraft = $state('');
	let draftNodeId = $state<string | null>(null);

	// Persist text fields on blur so a query refetch cannot replace the xyflow
	// node after every keypress and clear the current selection.
	$effect(() => {
		if (node.id !== draftNodeId) {
			draftNodeId = node.id;
			labelDraft = node.label ?? '';
			badgeDraft = node.badge_text ?? '';
		}
	});

	function handleFileChange(e: Event) {
		const file = (e.target as HTMLInputElement).files?.[0];
		if (file) onUploadImage(file);
	}

	function commitLabel() {
		if (labelDraft !== (node.label ?? '')) onUpdate({ label: labelDraft });
	}

	function commitBadge() {
		if (badgeDraft !== (node.badge_text ?? '')) onUpdate({ badge_text: badgeDraft });
	}

	function handleDraftKeydown(event: KeyboardEvent, reset: () => void) {
		event.stopPropagation();
		if (event.key === 'Enter') {
			event.preventDefault();
			(event.currentTarget as HTMLInputElement).blur();
		} else if (event.key === 'Escape') {
			event.preventDefault();
			reset();
			(event.currentTarget as HTMLInputElement).blur();
		}
	}

	function handleFontSizeChange(event: Event) {
		const value = Number((event.target as HTMLInputElement).value);
		if (Number.isInteger(value) && value >= 10 && value <= 72) {
			onUpdate({ font_size: value });
		}
	}
</script>

<div
	class="absolute right-2 top-2 z-10 max-h-[calc(100%-1rem)] w-64 space-y-3 overflow-y-auto rounded-md bg-white p-3 shadow-lg dark:bg-gray-900"
	role="dialog"
	aria-label={`${node.kind} node settings`}
	tabindex="-1"
	onpointerdown={(event) => event.stopPropagation()}
	onkeydown={(event) => event.stopPropagation()}
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
				value={labelDraft}
				oninput={(e) => (labelDraft = (e.target as HTMLInputElement).value)}
				onblur={commitLabel}
				onkeydown={(event) => handleDraftKeydown(event, () => (labelDraft = node.label ?? ''))}
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
					value={badgeDraft}
					oninput={(e) => (badgeDraft = (e.target as HTMLInputElement).value)}
					onblur={commitBadge}
					onkeydown={(event) =>
						handleDraftKeydown(event, () => (badgeDraft = node.badge_text ?? ''))}
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

	<div>
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
	</div>

	<div class="grid grid-cols-[1fr_5rem] gap-2">
		<label class="block text-xs font-medium">
			Font
			<select
				class="input-field mt-1 w-full"
				value={node.font_family ?? 'Sans'}
				onchange={(event) =>
					onUpdate({ font_family: (event.target as HTMLSelectElement).value as TextFont })}
			>
				{#each TEXT_FONTS as font (font.value)}
					<option value={font.value}>{font.label}</option>
				{/each}
			</select>
		</label>
		<label class="block text-xs font-medium">
			Size
			<input
				class="input-field mt-1 w-full"
				type="number"
				min="10"
				max="72"
				step="1"
				value={node.font_size ?? 16}
				onchange={handleFontSizeChange}
			/>
		</label>
	</div>
	<label class="block text-xs font-medium">
		Font style
		<select
			class="input-field mt-1 w-full"
			value={node.font_style ?? 'Normal'}
			onchange={(event) =>
				onUpdate({ font_style: (event.target as HTMLSelectElement).value as FontStyle })}
		>
			{#each FONT_STYLES as style (style)}<option value={style}>{style}</option>{/each}
		</select>
	</label>
	<label class="block text-xs font-medium">
		Border
		<select
			class="input-field mt-1 w-full"
			value={node.border_style ?? 'Solid'}
			onchange={(event) =>
				onUpdate({ border_style: (event.target as HTMLSelectElement).value as BorderStyle })}
		>
			{#each BORDER_STYLES as style (style)}<option value={style}>{style}</option>{/each}
		</select>
	</label>
	<label class="block text-xs font-medium">
		Transparency ({100 - (node.opacity ?? 100)}%)
		<input
			class="mt-1 w-full"
			type="range"
			min="0"
			max="100"
			value={node.opacity ?? 100}
			oninput={(event) => onUpdate({ opacity: Number((event.target as HTMLInputElement).value) })}
		/>
	</label>
	<label class="block text-xs font-medium">
		Link URL
		<input
			class="input-field mt-1 w-full"
			type="url"
			value={node.link_url ?? ''}
			placeholder="https://…"
			onchange={(event) => onUpdate({ link_url: (event.target as HTMLInputElement).value || null })}
		/>
	</label>

	{#if node.kind === 'Group' || node.kind === 'Entity' || node.kind === 'Library' || node.kind === 'Text'}
		<div>
			<span class="block text-xs font-medium">Primary color</span>
			<div class="mt-1 grid grid-cols-6 gap-1">
				{#each COLORS as color (color)}
					<button
						class="h-5 w-5 rounded-full border"
						class:ring-2={(node.primary_color ?? node.color) === color}
						style:background-color={createColorHelper(color).rgb}
						title={color}
						onclick={() => onUpdate({ primary_color: color, color })}
					></button>
				{/each}
			</div>
		</div>
	{/if}
	{#each [['Secondary color', 'secondary_color'], ['Background color', 'background_color']] as option (option[1])}
		<div>
			<span class="block text-xs font-medium">{option[0]}</span>
			<div class="mt-1 grid grid-cols-6 gap-1">
				{#each COLORS as color (color)}
					<button
						class="h-5 w-5 rounded-full border"
						class:ring-2={node[option[1] as 'secondary_color' | 'background_color'] === color}
						style:background-color={createColorHelper(color).rgb}
						title={color}
						onclick={() => onUpdate({ [option[1]]: color })}
					></button>
				{/each}
			</div>
		</div>
	{/each}
</div>

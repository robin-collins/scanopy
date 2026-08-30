<script lang="ts">
	import {
		Trash2,
		Upload,
		Bold,
		Italic,
		Underline,
		AlignLeft,
		AlignCenter,
		AlignRight
	} from 'lucide-svelte';
	import type {
		CustomViewNode,
		LibraryObject,
		NodeStyle,
		CornerStyle,
		TextAlign,
		BorderStyle,
		ServiceIconPosition,
		ServiceLabelVerticalAlign
	} from '$lib/features/custom-topology-views/queries';
	import type { components } from '$lib/api/schema';
	import { createColorHelper } from '$lib/shared/utils/styling';
	import {
		common_bold,
		common_delete,
		common_italic,
		common_underline,
		topology_customViewGroupInternalNamePlaceholder
	} from '$lib/paraglide/messages';
	import FontPicker from './FontPicker.svelte';
	import type { CanvasTypographyDefaults } from './custom-view-model';

	type Color = components['schemas']['Color'];

	interface Props {
		node: CustomViewNode;
		canvasDefaults: CanvasTypographyDefaults;
		libraryObjects: LibraryObject[];
		onUpdate: (patch: Partial<CustomViewNode>) => void;
		onUploadImage: (file: File) => void;
		onDelete: () => void;
	}

	let { node, canvasDefaults, libraryObjects, onUpdate, onUploadImage, onDelete }: Props = $props();

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

	const TEXT_ALIGNS: { value: TextAlign; Icon: typeof AlignLeft }[] = [
		{ value: 'Left', Icon: AlignLeft },
		{ value: 'Center', Icon: AlignCenter },
		{ value: 'Right', Icon: AlignRight }
	];
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
	let isServiceNode = $derived(node.kind === 'Entity' && node.entity_type === 'Service');
	let statsCardAvailable = $derived(node.kind === 'Entity' && node.entity_type === 'Host');
	let libraryObjectName = $derived(
		node.kind === 'Library'
			? (libraryObjects.find((o) => o.id === node.library_object_id)?.name ?? null)
			: null
	);
	let fileInput: HTMLInputElement | undefined = $state();
	let labelDraft = $state('');
	let textContentDraft = $state('');
	let badgeDraft = $state('');
	let nameDraft = $state('');
	let descriptionDraft = $state('');
	let draftNodeId = $state<string | null>(null);
	let labelSource = $state('');
	let textContentSource = $state('');
	let nameSource = $state('');
	let descriptionSource = $state('');
	let focusedMetadataDraft = $state<'label' | 'text_content' | 'name' | 'description' | null>(null);

	// Persist text fields on blur so a query refetch cannot replace the xyflow
	// node after every keypress and clear the current selection.
	$effect(() => {
		const nextLabel = node.label ?? '';
		const nextTextContent = node.text_content ?? '';
		const nextName = node.name ?? '';
		const nextDescription = node.description ?? '';
		if (node.id !== draftNodeId) {
			draftNodeId = node.id;
			labelDraft = nextLabel;
			textContentDraft = nextTextContent;
			badgeDraft = node.badge_text ?? '';
			nameDraft = nextName;
			descriptionDraft = nextDescription;
			labelSource = nextLabel;
			textContentSource = nextTextContent;
			nameSource = nextName;
			descriptionSource = nextDescription;
			focusedMetadataDraft = null;
			return;
		}

		// The group label can also be edited inline on the canvas. Absorb any
		// successful same-node update unless this exact inspector field is
		// actively being edited, then absorb it after the edit completes.
		if (nextLabel !== labelSource) {
			labelSource = nextLabel;
			if (focusedMetadataDraft !== 'label') labelDraft = nextLabel;
		}
		if (nextTextContent !== textContentSource) {
			textContentSource = nextTextContent;
			if (focusedMetadataDraft !== 'text_content') textContentDraft = nextTextContent;
		}
		if (nextName !== nameSource) {
			nameSource = nextName;
			if (focusedMetadataDraft !== 'name') nameDraft = nextName;
		}
		if (nextDescription !== descriptionSource) {
			descriptionSource = nextDescription;
			if (focusedMetadataDraft !== 'description') descriptionDraft = nextDescription;
		}
	});

	function handleFileChange(e: Event) {
		const file = (e.target as HTMLInputElement).files?.[0];
		if (file) onUploadImage(file);
	}

	function commitLabel() {
		if (labelDraft !== (node.label ?? '')) onUpdate({ label: labelDraft });
	}

	function commitTextContent() {
		if (textContentDraft !== (node.text_content ?? '')) {
			onUpdate({ text_content: textContentDraft });
		}
	}

	function commitBadge() {
		if (badgeDraft !== (node.badge_text ?? '')) onUpdate({ badge_text: badgeDraft });
	}

	function commitName() {
		if (nameDraft !== (node.name ?? '')) onUpdate({ name: nameDraft });
	}

	function commitDescription() {
		if (descriptionDraft !== (node.description ?? '')) onUpdate({ description: descriptionDraft });
	}

	function finishMetadataDraft(
		field: 'label' | 'text_content' | 'name' | 'description',
		commit: () => void
	) {
		commit();
		if (focusedMetadataDraft === field) focusedMetadataDraft = null;
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
		if (Number.isSafeInteger(value) && value >= 10) {
			onUpdate({ font_size: value });
		}
	}

	function booleanOverride(event: Event): boolean | null {
		const value = (event.target as HTMLSelectElement).value;
		return value === '' ? null : value === 'true';
	}

	function updateServiceOffset(
		field: 'service_label_offset_x' | 'service_label_offset_y',
		event: Event
	) {
		const value = Number((event.target as HTMLInputElement).value);
		if (!Number.isSafeInteger(value) || value < -1000 || value > 1000) return;
		if (field === 'service_label_offset_x') onUpdate({ service_label_offset_x: value });
		else onUpdate({ service_label_offset_y: value });
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
				maxlength={200}
				value={labelDraft}
				oninput={(e) => (labelDraft = (e.target as HTMLInputElement).value)}
				onfocus={() => (focusedMetadataDraft = 'label')}
				onblur={() => finishMetadataDraft('label', commitLabel)}
				onkeydown={(event) => handleDraftKeydown(event, () => (labelDraft = node.label ?? ''))}
			/>
		</label>
	{/if}

	{#if node.kind === 'Text'}
		<label class="block text-xs font-medium">
			Content
			<textarea
				class="input-field mt-1 w-full"
				rows="4"
				maxlength={5000}
				value={textContentDraft}
				oninput={(event) => (textContentDraft = (event.target as HTMLTextAreaElement).value)}
				onfocus={() => (focusedMetadataDraft = 'text_content')}
				onblur={() => finishMetadataDraft('text_content', commitTextContent)}
			></textarea>
		</label>
	{/if}

	{#if node.kind === 'Group'}
		<label class="block text-xs font-medium">
			Label
			<input
				class="input-field mt-1 w-full"
				maxlength={200}
				value={labelDraft}
				oninput={(e) => (labelDraft = (e.target as HTMLInputElement).value)}
				onfocus={() => (focusedMetadataDraft = 'label')}
				onblur={() => finishMetadataDraft('label', commitLabel)}
				onkeydown={(event) => handleDraftKeydown(event, () => (labelDraft = node.label ?? ''))}
			/>
		</label>
		<label class="block text-xs font-medium">
			Name
			<input
				class="input-field mt-1 w-full"
				maxlength={200}
				placeholder={topology_customViewGroupInternalNamePlaceholder()}
				value={nameDraft}
				oninput={(e) => (nameDraft = (e.target as HTMLInputElement).value)}
				onfocus={() => (focusedMetadataDraft = 'name')}
				onblur={() => finishMetadataDraft('name', commitName)}
				onkeydown={(event) => handleDraftKeydown(event, () => (nameDraft = node.name ?? ''))}
			/>
		</label>
		<label class="block text-xs font-medium">
			Description
			<textarea
				class="input-field mt-1 w-full"
				rows="2"
				maxlength={2000}
				value={descriptionDraft}
				oninput={(e) => (descriptionDraft = (e.target as HTMLTextAreaElement).value)}
				onfocus={() => (focusedMetadataDraft = 'description')}
				onblur={() => finishMetadataDraft('description', commitDescription)}
			></textarea>
		</label>
		<div class="flex gap-3">
			<label class="flex items-center gap-1.5 text-xs">
				<input
					type="checkbox"
					checked={node.show_label ?? true}
					onchange={(e) => onUpdate({ show_label: (e.target as HTMLInputElement).checked })}
				/> Show label
			</label>
			<label class="flex items-center gap-1.5 text-xs">
				<input
					type="checkbox"
					checked={node.show_description ?? true}
					onchange={(e) => onUpdate({ show_description: (e.target as HTMLInputElement).checked })}
				/> Show description
			</label>
		</div>
	{/if}

	{#if isServiceNode}
		<div class="space-y-2 rounded border p-2">
			<span class="block text-xs font-semibold">Service icon and label</span>
			<label class="flex items-center gap-2 text-xs">
				<input
					type="checkbox"
					checked={node.show_service_icon ?? true}
					onchange={(event) =>
						onUpdate({ show_service_icon: (event.target as HTMLInputElement).checked })}
				/>
				Show service icon
			</label>
			<label class="block text-xs font-medium">
				Icon position
				<select
					class="input-field mt-1 w-full"
					value={node.service_icon_position ?? 'BeforeName'}
					onchange={(event) =>
						onUpdate({
							service_icon_position: (event.target as HTMLSelectElement)
								.value as ServiceIconPosition
						})}
				>
					<option value="BeforeName">Before name</option>
					<option value="AfterName">After name</option>
					<option value="Center">Centre of object</option>
				</select>
			</label>
			<label class="block text-xs font-medium">
				Custom icon URL
				<div class="mt-1 flex gap-1">
					<input
						class="input-field min-w-0 flex-1"
						type="url"
						maxlength={2048}
						value={node.service_icon_url ?? ''}
						placeholder="https://…"
						onchange={(event) => {
							const value = (event.target as HTMLInputElement).value.trim();
							onUpdate({ service_icon_url: value || null });
						}}
					/>
					<button
						type="button"
						class="btn-secondary px-2 text-xs"
						disabled={!node.service_icon_url}
						onclick={() => onUpdate({ service_icon_url: null })}>Reset</button
					>
				</div>
			</label>
			<div class="grid grid-cols-2 gap-2">
				<label class="block text-xs font-medium">
					Horizontal
					<select
						class="input-field mt-1 w-full"
						value={node.service_label_horizontal_align ?? 'Center'}
						onchange={(event) =>
							onUpdate({
								service_label_horizontal_align: (event.target as HTMLSelectElement)
									.value as TextAlign
							})}
					>
						<option value="Left">Left</option>
						<option value="Center">Centre</option>
						<option value="Right">Right</option>
					</select>
				</label>
				<label class="block text-xs font-medium">
					Vertical
					<select
						class="input-field mt-1 w-full"
						value={node.service_label_vertical_align ?? 'Bottom'}
						onchange={(event) =>
							onUpdate({
								service_label_vertical_align: (event.target as HTMLSelectElement)
									.value as ServiceLabelVerticalAlign
							})}
					>
						<option value="Top">Top</option>
						<option value="Middle">Middle</option>
						<option value="Bottom">Bottom</option>
					</select>
				</label>
				<label class="block text-xs font-medium">
					X offset
					<input
						class="input-field mt-1 w-full"
						type="number"
						min="-1000"
						max="1000"
						step="1"
						value={node.service_label_offset_x ?? 0}
						onchange={(event) => updateServiceOffset('service_label_offset_x', event)}
					/>
				</label>
				<label class="block text-xs font-medium">
					Y offset
					<input
						class="input-field mt-1 w-full"
						type="number"
						min="-1000"
						max="1000"
						step="1"
						value={node.service_label_offset_y ?? 0}
						onchange={(event) => updateServiceOffset('service_label_offset_y', event)}
					/>
				</label>
			</div>
		</div>
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

	<FontPicker
		value={node.font_family ?? null}
		onSelect={(fontId) => onUpdate({ font_family: fontId })}
	/>
	<div>
		<label class="block text-xs font-medium">
			Size
			<input
				class="input-field mt-1 w-full"
				type="number"
				min="10"
				step="1"
				value={node.font_size ?? 16}
				onchange={handleFontSizeChange}
			/>
		</label>
	</div>
	<div class="grid grid-cols-3 gap-1">
		<label class="block text-xs font-medium">
			<span class="flex items-center gap-1"><Bold class="h-3.5 w-3.5" /> {common_bold()}</span>
			<select
				class="input-field mt-1 w-full"
				value={node.font_bold == null ? '' : String(node.font_bold)}
				onchange={(event) => onUpdate({ font_bold: booleanOverride(event) })}
			>
				<option value="">Canvas ({canvasDefaults.fontBold ? 'On' : 'Off'})</option>
				<option value="true">On</option>
				<option value="false">Off</option>
			</select>
		</label>
		<label class="block text-xs font-medium">
			<span class="flex items-center gap-1"><Italic class="h-3.5 w-3.5" /> {common_italic()}</span>
			<select
				class="input-field mt-1 w-full"
				value={node.font_italic == null ? '' : String(node.font_italic)}
				onchange={(event) => onUpdate({ font_italic: booleanOverride(event) })}
			>
				<option value="">Canvas ({canvasDefaults.fontItalic ? 'On' : 'Off'})</option>
				<option value="true">On</option>
				<option value="false">Off</option>
			</select>
		</label>
		<label class="block text-xs font-medium">
			<span class="flex items-center gap-1"
				><Underline class="h-3.5 w-3.5" /> {common_underline()}</span
			>
			<select
				class="input-field mt-1 w-full"
				value={node.font_underline == null ? '' : String(node.font_underline)}
				onchange={(event) => onUpdate({ font_underline: booleanOverride(event) })}
			>
				<option value="">Canvas ({canvasDefaults.fontUnderline ? 'On' : 'Off'})</option>
				<option value="true">On</option>
				<option value="false">Off</option>
			</select>
		</label>
	</div>
	<label class="block text-xs font-medium">
		Text align
		<select
			class="input-field mt-1 w-full"
			value={node.text_align ?? ''}
			onchange={(event) =>
				onUpdate({
					text_align: ((event.target as HTMLSelectElement).value || null) as TextAlign | null
				})}
		>
			<option value="">Canvas ({canvasDefaults.textAlign ?? 'Left'})</option>
			{#each TEXT_ALIGNS as opt (opt.value)}
				<option value={opt.value}>{opt.value}</option>
			{/each}
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
			<span class="block text-xs font-medium">Text colour</span>
			<div class="mt-1 grid grid-cols-6 gap-1">
				<button
					type="button"
					class="col-span-2 flex h-5 items-center justify-center rounded border text-[10px]"
					class:ring-2={node.text_color == null}
					title={`Canvas default (${canvasDefaults.textColor ?? 'Gray'})`}
					onclick={() => onUpdate({ text_color: null })}
				>
					Canvas
				</button>
				{#each COLORS as color (color)}
					<button
						type="button"
						class="h-5 w-5 rounded-full border"
						class:ring-2={node.text_color === color}
						style:background-color={createColorHelper(color).rgb}
						title={color}
						onclick={() => onUpdate({ text_color: color })}
					></button>
				{/each}
			</div>
		</div>
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

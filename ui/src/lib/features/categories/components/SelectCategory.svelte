<script lang="ts">
	import { useCategoriesQuery } from '$lib/features/categories/queries';
	import { common_category, common_unassigned } from '$lib/paraglide/messages';

	/**
	 * SelectCategory supports two usage patterns, mirroring SelectNetwork:
	 *
	 * 1. Binding: <SelectCategory bind:selectedCategoryId />
	 * 2. Callback: <SelectCategory selectedCategoryId={value} onCategoryChange={(id) => ...} />
	 *
	 * Unlike SelectNetwork, there is no auto-select-first — "Uncategorized"
	 * (null) is a valid, common resting state for a host.
	 */
	interface Props {
		selectedCategoryId?: string | null;
		disabled?: boolean;
		onCategoryChange?: (categoryId: string | null) => void;
	}

	let {
		selectedCategoryId = $bindable(null),
		disabled = false,
		onCategoryChange
	}: Props = $props();

	const categoriesQuery = useCategoriesQuery();
	let categories = $derived(
		[...(categoriesQuery.data ?? [])].sort((a, b) => a.name.localeCompare(b.name))
	);

	function handleChange(event: Event) {
		const value = (event.target as HTMLSelectElement).value || null;
		if (onCategoryChange) {
			onCategoryChange(value);
		} else {
			selectedCategoryId = value;
		}
	}
</script>

<div>
	<label for="category" class="text-secondary mb-2 block text-sm font-medium">
		{common_category()}
	</label>
	<select
		id="category"
		{disabled}
		value={selectedCategoryId ?? ''}
		onchange={handleChange}
		class="input-field"
	>
		<option class="select-option" value="">{common_unassigned()}</option>
		{#each categories as category (category.id)}
			<option class="select-option" value={category.id}>{category.name}</option>
		{/each}
	</select>
</div>

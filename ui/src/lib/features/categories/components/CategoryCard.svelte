<script lang="ts">
	import { Edit, Trash2, Lock } from 'lucide-svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import type { Category } from '../types/base';
	import { createColorHelper, createIconComponent } from '$lib/shared/utils/styling';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { permissions } from '$lib/shared/stores/metadata';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import {
		common_builtin,
		common_delete,
		common_description,
		common_edit,
		common_yes,
		common_no,
		categories_skipFullPortScan
	} from '$lib/paraglide/messages';

	let {
		category,
		onEdit = () => {},
		onDelete = () => {}
	}: {
		category: Category;
		onEdit?: (category: Category) => void;
		onDelete?: (category: Category) => void;
	} = $props();

	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	let colorHelper = $derived(createColorHelper(category.color));
	let Icon = $derived(createIconComponent(category.icon));
	let isBuiltin = $derived(!category.organization_id);

	let canManage = $derived(
		(currentUser && permissions.getMetadata(currentUser.permissions).manage_org_entities) || false
	);

	let fields = $derived([
		{ label: common_description(), value: category.description },
		{
			label: categories_skipFullPortScan(),
			value: category.skip_full_port_scan ? common_yes() : common_no()
		}
	]);

	let actions = $derived([
		...(canManage && !isBuiltin
			? [
					{
						label: common_delete(),
						icon: Trash2,
						class: 'btn-icon-danger',
						onClick: () => onDelete(category)
					}
				]
			: []),
		...(canManage
			? [
					{
						label: common_edit(),
						icon: Edit,
						onClick: () => onEdit(category)
					}
				]
			: [])
	]);
</script>

<div class="card flex h-full flex-col">
	<div class="mb-4 flex items-start">
		<div class="flex items-center space-x-3">
			{#if Icon}
				<Icon size={28} class={colorHelper.icon} />
			{/if}
			<div class="min-w-0 flex-1">
				<div class="flex items-center gap-2">
					<h3 class="text-primary min-w-0 truncate text-lg font-semibold">{category.name}</h3>
					{#if isBuiltin}
						<div class="flex-shrink-0">
							<Tag label={common_builtin()} icon={Lock} pill />
						</div>
					{/if}
				</div>
			</div>
		</div>
	</div>

	<div class="flex-grow space-y-3">
		{#each fields as field (field.label)}
			<div class="flex flex-wrap items-center gap-2 text-sm">
				<span class="text-secondary">{field.label}:</span>
				<span>{field.value}</span>
			</div>
		{/each}
	</div>

	{#if actions.length > 0}
		<div class="card-divider-h mt-4 flex items-center justify-between pt-4">
			{#each actions as action (action.label)}
				<button
					onclick={action.onClick}
					use:tooltip
					data-tooltip={action.label}
					aria-label={action.label}
					class="{action.class ?? 'btn-icon'} disabled:cursor-not-allowed disabled:opacity-50"
				>
					<action.icon size={16} class="flex-shrink-0" />
				</button>
			{/each}
		</div>
	{/if}
</div>

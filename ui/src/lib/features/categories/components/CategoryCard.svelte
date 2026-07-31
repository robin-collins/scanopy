<script lang="ts">
	import { Edit, Trash2, Lock } from 'lucide-svelte';
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import type { Category } from '../types/base';
	import { createColorHelper, createIconComponent } from '$lib/shared/utils/styling';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { permissions } from '$lib/shared/stores/metadata';
	import {
		common_delete,
		common_description,
		common_edit,
		common_yes,
		common_no,
		categories_skipFullPortScan,
		categories_builtin
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
	let isBuiltin = $derived(!category.organization_id);

	let canManage = $derived(
		(currentUser && permissions.getMetadata(currentUser.permissions).manage_org_entities) || false
	);

	let cardData = $derived({
		title: category.name,
		iconColor: colorHelper.icon,
		Icon: createIconComponent(category.icon),
		status: isBuiltin ? { label: categories_builtin(), icon: Lock, pill: true } : null,
		fields: [
			{
				label: common_description(),
				value: category.description
			},
			{
				label: categories_skipFullPortScan(),
				value: category.skip_full_port_scan ? common_yes() : common_no()
			}
		],
		actions: [
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
		]
	});
</script>

<GenericCard {...cardData} selectable={false} />

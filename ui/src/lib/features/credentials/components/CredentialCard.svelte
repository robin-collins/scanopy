<script lang="ts">
	import { Edit, Trash2 } from 'lucide-svelte';
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import type { Credential } from '../types/base';
	import { getCredentialSummary, getCredentialTypeId } from '../types/base';
	import { entities } from '$lib/shared/stores/metadata';
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { permissions } from '$lib/shared/stores/metadata';
	import { common_delete, common_edit, common_tags } from '$lib/paraglide/messages';

	let {
		credential,
		onEdit = () => {},
		onDelete = () => {},
		viewMode,
		selected,
		onSelectionChange = () => {}
	}: {
		credential: Credential;
		onEdit?: (credential: Credential) => void;
		onDelete?: (credential: Credential) => void;
		viewMode: 'card' | 'list';
		selected: boolean;
		onSelectionChange?: (selected: boolean) => void;
	} = $props();

	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	let colorHelper = $derived(entities.getColorHelper('Credential'));
	let typeId = $derived(getCredentialTypeId(credential));

	let canManage = $derived(
		(currentUser && permissions.getMetadata(currentUser.permissions).manage_org_entities) || false
	);

	let cardData = $derived({
		title: credential.name,
		iconColor: colorHelper.icon,
		Icon: entities.getIconComponent('Credential'),
		fields: [
			{
				label: 'Type',
				value: [
					{
						id: 'type',
						label: credentialTypes.getName(typeId),
						color: credentialTypes.getColorHelper(typeId).color
					}
				]
			},
			{
				label: 'Info',
				value: [
					{
						id: 'summary',
						label: getCredentialSummary(credential),
						color: colorHelper.color
					}
				]
			},
			{
				label: common_tags(),
				snippet: tagsSnippet
			}
		],
		actions: [
			...(canManage
				? [
						{
							label: common_delete(),
							icon: Trash2,
							class: 'btn-icon-danger',
							onClick: () => onDelete(credential)
						},
						{
							label: common_edit(),
							icon: Edit,
							onClick: () => onEdit(credential)
						}
					]
				: [])
		]
	});
</script>

{#snippet tagsSnippet()}
	<div class="flex items-center gap-2">
		<span class="text-secondary text-sm">{common_tags()}:</span>
		<TagPickerInline
			selectedTagIds={credential.tags}
			entityId={credential.id}
			entityType="Credential"
		/>
	</div>
{/snippet}

<GenericCard {...cardData} {viewMode} {selected} {onSelectionChange} selectable={canManage} />

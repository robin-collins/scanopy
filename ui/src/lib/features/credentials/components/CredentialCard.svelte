<script lang="ts">
	import { Edit, Trash2 } from 'lucide-svelte';
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import type { Credential } from '../types/base';
	import { getCredentialTypeId } from '../types/base';
	import { entities } from '$lib/shared/stores/metadata';
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { entityRef } from '$lib/shared/components/data/types';
	import type { Color } from '$lib/shared/utils/styling';
	import { permissions } from '$lib/shared/stores/metadata';
	import type { Network } from '$lib/features/networks/types';
	import type { Host } from '$lib/features/hosts/types/base';
	import {
		common_broadcast,
		common_delete,
		common_edit,
		common_hosts,
		common_networks,
		common_perHost,
		common_scope,
		common_tags,
		credentials_notAssigned,
		credentials_scopeBroadcastTooltip,
		credentials_scopePerHostTooltip
	} from '$lib/paraglide/messages';

	let {
		credential,
		assignedNetworks = [],
		assignedHosts = [],
		onEdit = () => {},
		onDelete = () => {},
		viewMode,
		selected,
		onSelectionChange = () => {}
	}: {
		credential: Credential;
		assignedNetworks?: Network[];
		assignedHosts?: Host[];
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
	let scopeModels = $derived(credentialTypes.getMetadata(typeId)?.scope_models ?? []);

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
				label: common_scope(),
				value: scopeModels.map((s) => ({
					id: s,
					label: s === 'Broadcast' ? common_broadcast() : common_perHost(),
					title:
						s === 'Broadcast'
							? credentials_scopeBroadcastTooltip()
							: credentials_scopePerHostTooltip(),
					color: (s === 'Broadcast' ? 'Cyan' : 'Purple') as Color
				}))
			},
			...(scopeModels.includes('Broadcast')
				? [
						{
							label: common_networks(),
							value:
								assignedNetworks.length > 0
									? assignedNetworks.map((n) => ({
											id: n.id,
											label: n.name,
											color: entities.getColorHelper('Network').color as Color,
											entityRef: entityRef('Network', n.id, n)
										}))
									: [{ id: 'none', label: credentials_notAssigned(), color: 'Gray' as Color }]
						}
					]
				: []),
			...(scopeModels.includes('PerHost')
				? [
						{
							label: common_hosts(),
							value:
								assignedHosts.length > 0
									? assignedHosts.map((h) => ({
											id: h.id,
											label: h.name ?? h.id,
											color: entities.getColorHelper('Host').color as Color,
											entityRef: entityRef('Host', h.id, h)
										}))
									: [{ id: 'none', label: credentials_notAssigned(), color: 'Gray' as Color }]
						}
					]
				: []),
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

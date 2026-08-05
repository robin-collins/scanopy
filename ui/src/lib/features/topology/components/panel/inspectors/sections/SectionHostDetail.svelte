<script lang="ts">
	import type { Node } from '@xyflow/svelte';
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import type { RenderableTopology } from '$lib/features/topology/types/base';
	import type { TopologyEditState } from '$lib/features/topology/state';
	import type { ElementRenderContext } from '$lib/features/topology/resolvers';
	import { useUpdateHostDescriptionMutation } from '$lib/features/hosts/queries';
	import { useCategoriesQuery } from '$lib/features/categories/queries';
	import { hostOsGroups } from '$lib/shared/stores/metadata';
	import InfoRow from '$lib/shared/components/data/InfoRow.svelte';
	import {
		common_category,
		common_manufacturer,
		common_model,
		hosts_details_osGroup,
		inspector_hostDetail
	} from '$lib/paraglide/messages';

	/* eslint-disable @typescript-eslint/no-unused-vars -- component contract props */
	let {
		node,
		topology,
		editState,
		elementContext
	}: {
		node: Node;
		topology: RenderableTopology;
		editState: TopologyEditState;
		elementContext?: ElementRenderContext;
	} = $props();
	/* eslint-enable @typescript-eslint/no-unused-vars */

	let isReadonly = $derived(editState.isReadonly);
	let host = $derived(elementContext?.host ?? null);

	const updateHostDescriptionMutation = useUpdateHostDescriptionMutation();

	const categoriesQuery = useCategoriesQuery();
	let categoryName = $derived(
		host?.category_id
			? (categoriesQuery.data?.find((c) => c.id === host?.category_id)?.name ?? null)
			: null
	);
	let osLabel = $derived.by(() => {
		if (!host) return null;
		const group = host.os_group ? hostOsGroups.getName(host.os_group) : null;
		if (group && host.os_detail) return `${group} (${host.os_detail})`;
		return group ?? host.os_detail ?? null;
	});
	let makeModel = $derived(
		host && (host.manufacturer || host.model)
			? [host.manufacturer, host.model].filter(Boolean).join(' ')
			: null
	);
	let hasMetadata = $derived(!!(categoryName || osLabel || makeModel));

	let hostContext = $derived({
		services: topology.services.filter((s) => host && s.host_id === host.id),
		showEntityTagPicker: true,
		tagPickerDisabled: !editState.isEditable,
		entityTags: isReadonly ? (topology.entity_tags ?? []) : undefined,
		showEditableEntityDescription: true,
		entityDescription: host?.description ?? null,
		entityDescriptionDisabled: !editState.isEditable,
		onEntityDescriptionSave: (desc: string | null) => {
			if (host) {
				updateHostDescriptionMutation.mutate({ host, description: desc });
			}
		},
		compact: true
	});
</script>

{#if host}
	<div>
		<span class="text-secondary mb-2 block text-sm font-medium">{inspector_hostDetail()}</span>
		<div class="card card-static">
			<EntityDisplayWrapper context={hostContext} item={host} displayComponent={HostDisplay} />
			{#if hasMetadata}
				<div class="mt-3 space-y-1 border-t border-gray-700/50 pt-3">
					{#if categoryName}
						<InfoRow label={common_category()}>{categoryName}</InfoRow>
					{/if}
					{#if osLabel}
						<InfoRow label={hosts_details_osGroup()}>{osLabel}</InfoRow>
					{/if}
					{#if makeModel}
						<InfoRow label={`${common_manufacturer()} / ${common_model()}`}>{makeModel}</InfoRow>
					{/if}
				</div>
			{/if}
		</div>
	</div>
{/if}

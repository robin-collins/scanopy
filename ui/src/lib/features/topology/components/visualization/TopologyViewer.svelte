<script lang="ts">
	import { type Node } from '@xyflow/svelte';
	import {
		topologyReadOnly,
		selectedEdge,
		selectedNode,
		selectedNodes,
		useUpdateNodePositionMutation,
		useResetNodePositionsMutation
	} from '../../queries';
	import { type RenderableTopology } from '../../types/base';
	import { searchOpen } from '../../interactions';
	import { editModeEnabled } from '../../state';
	import { createTopologyKeydownHandler } from '../../keyboard';
	import { snapPositionToGrid } from '../../layout/layout-overrides';
	import BaseTopologyViewer from './BaseTopologyViewer.svelte';
	import SearchOverlay from './SearchOverlay.svelte';
	import ShortcutsHelpOverlay from './ShortcutsHelpOverlay.svelte';
	import { onDestroy } from 'svelte';
	import { queryClient, queryKeys } from '$lib/api/query-client';
	import { pushError } from '$lib/shared/stores/feedback';
	import { topology_layoutSaveFailed } from '$lib/paraglide/messages';

	// Props for callbacks from parent
	let {
		topology,
		onRebuild,
		isActive = false
	}: {
		topology: RenderableTopology | null | undefined;
		onRebuild?: () => void;
		isActive?: boolean;
	} = $props();

	const updateNodePositionMutation = useUpdateNodePositionMutation();
	const resetNodePositionsMutation = useResetNodePositionsMutation();

	let baseViewer: BaseTopologyViewer | null = $state(null);

	// Overlay state
	let shortcutsHelpOpen = $state(false);

	let editMode = $state(false);

	function toggleEditMode() {
		if ($topologyReadOnly) return;
		editMode = !editMode;
		editModeEnabled.set(editMode);
	}

	// Force view mode whenever the topology becomes read-only (e.g. selecting a
	// snapshot while in edit mode).
	$effect(() => {
		if ($topologyReadOnly && editMode) {
			editMode = false;
			editModeEnabled.set(false);
		}
	});

	// Sidebar buttons show labels briefly on first visit per session, then stay collapsed
	const SIDEBAR_SEEN_KEY = 'topology_sidebar_labels_shown';
	const alreadySeen =
		typeof sessionStorage !== 'undefined' && sessionStorage.getItem(SIDEBAR_SEEN_KEY) === '1';
	let sidebarCollapsed = $state(alreadySeen);

	$effect(() => {
		if (isActive && !alreadySeen && !sidebarCollapsed) {
			const timer = setTimeout(() => {
				sidebarCollapsed = true;
				sessionStorage.setItem(SIDEBAR_SEEN_KEY, '1');
			}, 2000);
			return () => clearTimeout(timer);
		}
	});

	// Reset edit mode when leaving this tab (tabs stay mounted, just hidden)
	$effect(() => {
		if (!isActive && editMode) {
			editMode = false;
			editModeEnabled.set(false);
		}
	});

	onDestroy(() => {
		editModeEnabled.set(false);
	});

	export function triggerFitView() {
		baseViewer?.triggerFitView();
	}

	async function refetchLayout() {
		if (!topology) return;
		await queryClient.invalidateQueries({
			queryKey: queryKeys.topology.data(topology.network_id, undefined)
		});
	}

	async function handleNodeDragStop(targetNode: Node) {
		if (!topology || $topologyReadOnly || !editMode) return;
		const movedNode = topology.nodes.find((node) => node.id === targetNode.id);
		if (!movedNode) return;

		const position = snapPositionToGrid(targetNode.position);
		const previousPosition = baseViewer?.getNodePosition(movedNode.id);
		baseViewer?.setNodePosition(movedNode.id, position);
		try {
			await updateNodePositionMutation.mutateAsync({
				topologyId: topology.id,
				view: topology.view,
				nodeId: movedNode.id,
				position
			});
			await refetchLayout();
		} catch {
			if (previousPosition) baseViewer?.setNodePosition(movedNode.id, previousPosition);
			pushError(topology_layoutSaveFailed());
			await refetchLayout();
		}
	}

	async function handleResetLayout() {
		if (!topology || $topologyReadOnly) return;
		try {
			await resetNodePositionsMutation.mutateAsync({
				topologyId: topology.id,
				view: topology.view
			});
			await refetchLayout();
			baseViewer?.resetAutomaticLayout();
		} catch {
			pushError(topology_layoutSaveFailed());
			await refetchLayout();
		}
	}

	const handleKeydown = createTopologyKeydownHandler({
		getBaseViewer: () => baseViewer,
		getShortcutsHelpOpen: () => shortcutsHelpOpen,
		setShortcutsHelpOpen: (open) => (shortcutsHelpOpen = open),
		selectionStores: { selectedNode, selectedEdge, selectedNodes },
		isEnabled: () => isActive,
		onToggleEdit: toggleEditMode,
		onRebuild: () => onRebuild?.()
	});
</script>

<svelte:window onkeydown={handleKeydown} />

{#if topology}
	<div class="relative h-[calc(100vh-120px)] w-full">
		<BaseTopologyViewer
			bind:this={baseViewer}
			{topology}
			readonly={!editMode || $topologyReadOnly}
			showControls={true}
			{editMode}
			{sidebarCollapsed}
			onToggleEditMode={$topologyReadOnly ? null : toggleEditMode}
			onResetLayout={$topologyReadOnly ? null : handleResetLayout}
			resetLayoutDisabled={resetNodePositionsMutation.isPending}
			onNodeDragStop={handleNodeDragStop}
			onOpenShortcuts={() => (shortcutsHelpOpen = true)}
			onOpenSearch={() => searchOpen.set(true)}
		/>
		<SearchOverlay />
		<ShortcutsHelpOverlay bind:isOpen={shortcutsHelpOpen} readonly={$topologyReadOnly} />
	</div>
{/if}

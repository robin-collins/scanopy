<script lang="ts">
	import { writable, get } from 'svelte/store';
	import {
		SvelteFlow,
		Controls,
		MiniMap,
		Panel,
		Background,
		BackgroundVariant,
		type EdgeMarkerType,
		useNodesInitialized,
		type Connection,
		useSvelteFlow
	} from '@xyflow/svelte';
	import { Keyboard } from 'lucide-svelte';
	import { topology_shortcutsTitle } from '$lib/paraglide/messages';
	import { type Node, type Edge } from '@xyflow/svelte';
	import '@xyflow/svelte/dist/style.css';
	import { edgeTypes } from '$lib/shared/stores/metadata';
	import { pushError } from '$lib/shared/stores/feedback';
	import { previewEdges, selectedNodes, topologyOptions } from '../../queries';
	import { isExporting } from '../../interactions';

	// Import custom node/edge components
	import SubnetNode from './SubnetNode.svelte';
	import InterfaceNode from './InterfaceNode.svelte';
	import CustomEdge from './CustomEdge.svelte';
	import type { TopologyEdge, Topology } from '../../types/base';
	import { updateConnectedNodes, toggleEdgeHover, getEdgeDisplayState } from '../../interactions';
	import { onMount, tick, setContext } from 'svelte';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { writable as svelteWritable } from 'svelte/store';
	import { themeStore } from '$lib/shared/stores/theme.svelte';

	// Props
	export let topology: Topology;
	export let readonly: boolean = false;
	export let showControls: boolean = true;
	export let isEmbed: boolean = false;
	export let showBranding: boolean = false;
	export let showMinimap: boolean | undefined = undefined;

	// Create a context store for the topology so child nodes can access it
	const topologyContext = svelteWritable<Topology>(topology);
	setContext('topology', topologyContext);

	// Keep context in sync with prop
	$: topologyContext.set(topology);

	// Selection state - can be bound by parent
	export let selectedNode: Node | null = null;
	export let selectedEdge: Edge | null = null;

	// Optional callbacks for editing
	export let onNodeDragStop: ((node: Node) => void) | null = null;
	export let onReconnect: ((edge: Edge, newConnection: Connection) => void) | null = null;

	// Optional callbacks for selection changes
	export let onNodeSelect: ((node: Node | null, event?: MouseEvent | TouchEvent) => void) | null =
		null;
	export let onEdgeSelect: ((edge: Edge | null) => void) | null = null;
	export let onPaneSelect: ((event?: MouseEvent, wasPanning?: boolean) => void) | null = null;
	export let onSelectionChange: ((nodes: Node[], edges: Edge[]) => void) | null = null;
	export let onOpenShortcuts: (() => void) | null = null;

	// Track viewport panning state
	let viewportMoved = false;
	let viewportMoveTimer: ReturnType<typeof setTimeout> | null = null;

	const { fitView } = useSvelteFlow();
	const queryClient = useQueryClient();
	let containerElement: HTMLDivElement;

	export function triggerFitView() {
		requestAnimationFrame(() => fitView({ padding: 0.2 }));
	}

	export function fitViewToNodes(nodeIds: string[]) {
		requestAnimationFrame(() =>
			fitView({ nodes: nodeIds.map((id) => ({ id })), padding: 0.5, duration: 300 })
		);
	}

	onMount(() => {
		const { fitView } = useSvelteFlow();

		const observer = new IntersectionObserver(
			(entries) => {
				if (entries[0].isIntersecting) {
					requestAnimationFrame(() => fitView());
					observer.disconnect();
				}
			},
			{ threshold: 0.1 }
		);

		if (containerElement) {
			observer.observe(containerElement);
		}

		return () => observer.disconnect();
	});

	// Define node types
	const nodeTypes = {
		SubnetNode: SubnetNode,
		InterfaceNode: InterfaceNode
	};

	const customEdgeTypes = {
		custom: CustomEdge
	};

	// Stores for SvelteFlow
	let nodes = writable<Node[]>([]);
	let edges = writable<Edge[]>([]);

	// Hook to check when nodes are initialized
	const nodesInitialized = useNodesInitialized();

	// Store pending edges until nodes are ready
	let pendingEdges: Edge[] = [];

	// Load topology data when it changes
	$: if (topology && (topology.edges || topology.nodes)) {
		void loadTopologyData();
	}

	// Update edges when selection changes
	$: {
		void selectedNode;
		void selectedEdge;
		void $selectedNodes;

		if (topology && (topology.edges || topology.nodes)) {
			const currentEdges = get(edges);
			const currentNodes = get(nodes);
			const multiSelected = get(selectedNodes);
			updateConnectedNodes(
				selectedNode,
				selectedEdge,
				currentEdges,
				currentNodes,
				queryClient,
				topology,
				multiSelected
			);

			// Update edge animated state based on selection
			const updatedEdges = currentEdges.map((edge) => {
				const { shouldAnimate } = getEdgeDisplayState(edge, selectedNode, selectedEdge);

				return {
					...edge,
					id: edge.id, // Force new reference
					animated: shouldAnimate
				};
			});

			edges.set(updatedEdges);
		}
	}

	// Add edges when nodes are ready
	$: if (nodesInitialized.current && pendingEdges.length > 0) {
		edges.set(pendingEdges);
		pendingEdges = [];
	}

	async function loadTopologyData() {
		try {
			if (topology && (topology.edges || topology.nodes)) {
				// Create nodes FIRST
				const allNodes: Node[] = topology.nodes.map((node) => ({
					id: node.id,
					type: node.node_type,
					position: { x: node.position.x, y: node.position.y },
					width: node.size.x,
					height: node.size.y,
					expandParent: true,
					deletable: false,
					selectable: node.node_type !== 'SubnetNode',
					parentId: node.node_type == 'InterfaceNode' ? node.subnet_id : undefined,
					extent: node.node_type == 'InterfaceNode' ? 'parent' : undefined,
					data: node
				}));

				// Save current edge animated states before clearing
				const currentEdges = get(edges);
				const animatedStates = new Map(currentEdges.map((edge) => [edge.id, edge.animated]));

				// Clear edges FIRST
				edges.set([]);

				// Sort so children come before parents (as per Svelte Flow docs)
				const sortedNodes = allNodes.sort((a, b) => {
					if (a.parentId && !b.parentId) return 1; // children first
					if (!a.parentId && b.parentId) return -1; // parents second
					return 0;
				});

				// Set nodes
				nodes.set(sortedNodes);

				// Create edges with markers
				const flowEdges: Edge[] = topology.edges.map((edge: TopologyEdge, index: number) => {
					const edgeType = edge.edge_type as string;
					const edgeMetadata = edgeTypes.getMetadata(edgeType);
					const edgeColorHelper = edgeTypes.getColorHelper(edgeType);

					const markerStart = !edgeMetadata.has_start_marker
						? undefined
						: ({
								type: 'arrow',
								color: edgeColorHelper.rgb
							} as EdgeMarkerType);
					const markerEnd = !edgeMetadata.has_end_marker
						? undefined
						: ({
								type: 'arrow',
								color: edgeColorHelper.rgb
							} as EdgeMarkerType);

					const edgeId = `edge-${index}`;

					return {
						id: `edge-${index}`,
						source: edge.source,
						target: edge.target,
						markerEnd,
						markerStart,
						sourceHandle: edge.source_handle.toString(),
						targetHandle: edge.target_handle.toString(),
						type: 'custom',
						label: edge.label ?? undefined,
						data: { ...edge, edgeIndex: index },
						animated: animatedStates.get(edgeId) ?? false,
						interactionWidth: 50
					};
				});

				pendingEdges = flowEdges;

				// Wait for nodes to render, then set edges
				await tick();
				if (pendingEdges.length > 0) {
					edges.set(pendingEdges);
					pendingEdges = [];
				}
			}
		} catch (err) {
			pushError(`Failed to parse topology data ${err}`);
		}
	}

	function handleNodeDragStop({
		targetNode
	}: {
		targetNode: Node | null;
		nodes: Node[];
		event: MouseEvent | TouchEvent;
	}) {
		if (onNodeDragStop && targetNode) {
			onNodeDragStop(targetNode);
		}
	}

	function handleReconnect(edge: Edge, newConnection: Connection) {
		if (onReconnect) {
			onReconnect(edge, newConnection);
		}
	}

	function handleNodeClick({ node, event }: { node: Node; event: MouseEvent | TouchEvent }) {
		const isModifierClick = event instanceof MouseEvent && (event.ctrlKey || event.metaKey);

		if (!isModifierClick) {
			selectedNode = node;
			selectedEdge = null;
		}
		if (onNodeSelect) {
			onNodeSelect(node, event);
		}
	}

	function handleEdgeClick({ edge }: { edge: Edge; event: MouseEvent }) {
		selectedEdge = edge;
		selectedNode = null;
		if (onEdgeSelect) {
			onEdgeSelect(edge);
		}
	}

	function handleMove() {
		viewportMoved = true;
		if (viewportMoveTimer) {
			clearTimeout(viewportMoveTimer);
			viewportMoveTimer = null;
		}
	}

	function handleMoveEnd() {
		// Delay clearing the flag so it's still set when onpaneclick fires
		viewportMoveTimer = setTimeout(() => {
			viewportMoved = false;
		}, 50);
	}

	function handlePaneClick({ event }: { event: MouseEvent }) {
		selectedEdge = null;
		if (!viewportMoved) {
			selectedNode = null;
		}
		if (onPaneSelect) {
			onPaneSelect(event, viewportMoved);
		}
		// Reset immediately after handling
		viewportMoved = false;
		if (viewportMoveTimer) {
			clearTimeout(viewportMoveTimer);
			viewportMoveTimer = null;
		}
	}

	function handleEdgeHover({ edge }: { edge: Edge }) {
		const currentEdges = get(edges);
		toggleEdgeHover(edge, currentEdges);

		// Update animated state for all edges after hover toggle
		const updatedEdges = currentEdges.map((e) => {
			const { shouldAnimate } = getEdgeDisplayState(e, selectedNode, selectedEdge);

			return {
				...e,
				id: e.id,
				animated: shouldAnimate
			};
		});

		edges.set(updatedEdges);
	}

	function handleSelectionChange({ nodes: selNodes }: { nodes: Node[]; edges: Edge[] }) {
		if (onSelectionChange) {
			onSelectionChange(selNodes, []);
		}
	}

	// Merge preview edges into the edge store when they change
	$: {
		const preview = $previewEdges;
		if (preview.length > 0) {
			const currentEdges = get(edges);
			// Remove old preview edges, add new ones
			const realEdges = currentEdges.filter((e) => !e.id.startsWith('preview-'));
			edges.set([...realEdges, ...preview]);
		} else {
			const currentEdges = get(edges);
			const hasPreview = currentEdges.some((e) => e.id.startsWith('preview-'));
			if (hasPreview) {
				edges.set(currentEdges.filter((e) => !e.id.startsWith('preview-')));
			}
		}
	}
</script>

<div
	class="h-full w-full overflow-hidden !p-0"
	class:card={!isEmbed}
	class:card-static={!isEmbed}
	bind:this={containerElement}
>
	<SvelteFlow
		nodes={$nodes}
		edges={$edges}
		{nodeTypes}
		edgeTypes={customEdgeTypes}
		onpaneclick={handlePaneClick}
		onedgeclick={handleEdgeClick}
		onnodeclick={handleNodeClick}
		onedgepointerenter={handleEdgeHover}
		onedgepointerleave={handleEdgeHover}
		onnodedragstop={readonly ? undefined : handleNodeDragStop}
		onreconnect={readonly ? undefined : handleReconnect}
		onselectionchange={handleSelectionChange}
		onmove={handleMove}
		onmoveend={handleMoveEnd}
		fitView={true}
		minZoom={0.1}
		noPanClass="nopan"
		snapGrid={[25, 25]}
		nodesDraggable={!readonly}
		nodesConnectable={!readonly}
		elementsSelectable={true}
		selectionOnDrag={true}
		selectionKey="Shift"
		panOnDrag={true}
		zoomOnScroll={true}
	>
		<Background
			variant={BackgroundVariant.Dots}
			bgColor="var(--color-topology-bg)"
			gap={50}
			size={1}
		/>

		{#if showControls}
			<Panel position="top-right" class="!m-[10px] !flex !flex-col !items-center !gap-2 !p-0">
				{#if onOpenShortcuts}
					<button
						class="flex items-center justify-center rounded !border !border-gray-300 !bg-gray-50 p-1.5 !text-gray-700 !shadow-lg hover:!bg-gray-100 dark:!border-gray-600 dark:!bg-gray-700 dark:!text-gray-100 dark:hover:!bg-gray-600"
						onclick={onOpenShortcuts}
						title={topology_shortcutsTitle()}
					>
						<Keyboard class="h-4 w-4" />
					</button>
				{/if}
				<Controls
					showZoom={true}
					showFitView={true}
					showLock={false}
					class="!static !m-0 !rounded !border !border-gray-300 !bg-white !shadow-lg dark:!border-gray-600 dark:!bg-gray-800 [&_button:hover]:!bg-gray-100 dark:[&_button:hover]:!bg-gray-600 [&_button]:!border-gray-300 [&_button]:!bg-gray-50 [&_button]:!text-gray-700 dark:[&_button]:!border-gray-600 dark:[&_button]:!bg-gray-700 dark:[&_button]:!text-gray-100"
				/>
			</Panel>
		{/if}

		{#if (showMinimap !== undefined ? showMinimap : $topologyOptions.local.show_minimap) && !$isExporting}
			<MiniMap
				position="bottom-left"
				bgColor={themeStore.resolvedTheme === 'dark' ? '#1f2937' : '#ffffff'}
				nodeColor={themeStore.resolvedTheme === 'dark' ? '#6b7280' : '#9ca3af'}
				maskColor={themeStore.resolvedTheme === 'dark'
					? 'rgba(17, 24, 39, 0.7)'
					: 'rgba(243, 244, 246, 0.7)'}
				maskStrokeColor={themeStore.resolvedTheme === 'dark' ? '#374151' : '#d1d5db'}
			/>
		{/if}

		{#if showBranding}
			<a
				href="https://scanopy.net?utm_source={isEmbed
					? 'embed'
					: 'share'}&utm_medium=referral&utm_campaign=created_with"
				target="_blank"
				rel="noopener noreferrer"
				class="branding-badge"
			>
				<img src="/logos/scanopy-logo.png" alt="Scanopy" class="h-4 w-4" />
				<span>Created with Scanopy</span>
			</a>
		{/if}
	</SvelteFlow>
</div>

<style>
	:global(.svelte-flow__attribution) {
		background: transparent;
		color: var(--color-text-disabled);
		font-size: 10px;
	}

	:global(.svelte-flow__attribution.bottom) {
		bottom: 10px;
		right: 10px;
	}

	:global(.svelte-flow__attribution a) {
		color: var(--color-text-disabled);
	}

	:global(.svelte-flow__attribution a:hover) {
		color: var(--color-text-muted);
	}

	.branding-badge {
		position: absolute;
		bottom: 10px;
		right: 10px;
		display: flex;
		align-items: center;
		gap: 6px;
		color: var(--color-text-muted);
		font-size: 12px;
		text-decoration: none;
		z-index: 5;
		transition: color 0.2s;
	}

	.branding-badge:hover {
		color: var(--color-text-secondary);
	}

	:global(.hide-for-export .svelte-flow__attribution) {
		opacity: 0;
	}

	:global(.hide-for-export .svelte-flow__controls) {
		opacity: 0;
	}

	:global(.hide-for-export .svelte-flow__panel.top.right) {
		opacity: 0;
	}

	:global(.hide-for-export .svelte-flow__minimap) {
		opacity: 0;
	}

	:global(.hide-for-export .svelte-flow__resize-control) {
		opacity: 0;
	}

	:global(.hide-for-export .branding-badge) {
		opacity: 0;
	}

	/* Force full opacity on all nodes during export to disable focus effect */
	:global(.hide-for-export .svelte-flow__node .card) {
		opacity: 1 !important;
		transition: none !important;
	}

	:global(.hide-for-export .svelte-flow__node > .relative) {
		opacity: 1 !important;
		transition: none !important;
	}
</style>

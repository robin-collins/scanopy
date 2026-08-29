<script lang="ts">
	import {
		SvelteFlow,
		Background,
		BackgroundVariant,
		MiniMap,
		ConnectionMode,
		useSvelteFlow,
		type Node,
		type Edge,
		type NodeTypes,
		type EdgeTypes
	} from '@xyflow/svelte';
	import '@xyflow/svelte/dist/style.css';
	import CustomObjectNode from './CustomObjectNode.svelte';
	import CustomTextNode from './CustomTextNode.svelte';
	import CustomGroupNode from './CustomGroupNode.svelte';
	import CustomViewEdge from './CustomViewEdge.svelte';
	import CustomViewPalette from './CustomViewPalette.svelte';
	import CustomViewNodeInspector from './CustomViewNodeInspector.svelte';
	import type {
		CanvasNodeBounds,
		CustomObjectNodeData,
		CustomTextNodeData,
		CustomGroupNodeData
	} from './types';
	import {
		reconcileCompletedBoundsChange,
		type CanvasNodeGeometry,
		type CompletedBoundsChangeCause,
		type MembershipPatch
	} from './group-membership';
	import {
		useCustomViewNodesQuery,
		useCustomViewEdgesQuery,
		useCreateCustomViewNodeMutation,
		useUpdateCustomViewNodeMutation,
		useDeleteCustomViewNodeMutation,
		useCreateCustomViewEdgeMutation,
		useDeleteCustomViewEdgeMutation,
		useSaveCustomTopologyViewLayoutMutation,
		useDeleteCustomTopologyViewMutation,
		useUpdateCustomTopologyViewMutation,
		useCustomTopologyViewsQuery,
		useLibraryObjectsQuery,
		useUploadCustomViewNodeImageMutation,
		customViewNodeImageUrl,
		libraryObjectImageUrl,
		type CustomViewNode as CustomViewNodeRecord,
		type CustomViewEdge as CustomViewEdgeRecord
	} from '$lib/features/custom-topology-views/queries';
	import { useHostsQuery } from '$lib/features/hosts/queries';
	import { useServicesQuery } from '$lib/features/services/queries';
	import { useIPAddressesQuery } from '$lib/features/ip-addresses/queries';
	import { usePortsQuery } from '$lib/features/ports/queries';
	import { hostImageContentUrl } from '$lib/features/host-images/queries';
	import { createColorHelper } from '$lib/shared/utils/styling';
	import { getSafeCanvasLink, filterIdentifiedHosts } from './custom-view-model';
	import { loadFontsInUse } from './fonts';
	import CanvasControlPanel from './CanvasControlPanel.svelte';
	import { getCanvasContentBounds, type Rect } from './content-bounds';
	import { pushError } from '$lib/shared/stores/feedback';
	import { topology_customViewDeleteConfirm } from '$lib/paraglide/messages';

	interface Props {
		viewId: string;
		networkId: string;
		onClose: () => void;
	}

	let { viewId, networkId, onClose }: Props = $props();

	const nodesQuery = useCustomViewNodesQuery(() => viewId);
	const edgesQuery = useCustomViewEdgesQuery(() => viewId);
	const viewsQuery = useCustomTopologyViewsQuery(() => networkId);
	const hostsQuery = useHostsQuery(() => ({ network_id: networkId, limit: 0 }));
	const servicesQuery = useServicesQuery(() => ({ network_id: networkId, limit: 0 }));
	const libraryObjectsQuery = useLibraryObjectsQuery();
	const ipAddressesQuery = useIPAddressesQuery();
	const portsQuery = usePortsQuery();

	let currentView = $derived((viewsQuery.data ?? []).find((v) => v.id === viewId) ?? null);

	$effect(() => {
		loadFontsInUse([
			currentView?.default_font_family,
			...(nodesQuery.data ?? []).map((n) => n.font_family)
		]);
	});

	let identifiedHosts = $derived(
		filterIdentifiedHosts(
			hostsQuery.data?.items ?? [],
			ipAddressesQuery.data ?? [],
			servicesQuery.data?.items ?? []
		)
	);

	const createNodeMutation = useCreateCustomViewNodeMutation();
	const updateNodeMutation = useUpdateCustomViewNodeMutation();
	const deleteNodeMutation = useDeleteCustomViewNodeMutation();
	const createEdgeMutation = useCreateCustomViewEdgeMutation();
	const deleteEdgeMutation = useDeleteCustomViewEdgeMutation();
	const saveLayoutMutation = useSaveCustomTopologyViewLayoutMutation(() => viewId);
	const uploadNodeImageMutation = useUploadCustomViewNodeImageMutation();
	const deleteViewMutation = useDeleteCustomTopologyViewMutation();
	const updateViewMutation = useUpdateCustomTopologyViewMutation();

	const { screenToFlowPosition, getInternalNode, fitBounds } = useSvelteFlow();

	/** The group frame (if any) whose bounds contain a node's center point. Nested/overlapping groups resolve to the smallest. */
	function findEnclosingGroup(
		centerX: number,
		centerY: number,
		excludeNodeId?: string
	): CustomViewNodeRecord | null {
		const groups = (nodesQuery.data ?? []).filter(
			(v) => v.kind === 'Group' && v.id !== excludeNodeId
		);
		const containing = groups.filter((g) => {
			const width = g.width ?? 300;
			const height = g.height ?? 200;
			return centerX >= g.x && centerX <= g.x + width && centerY >= g.y && centerY <= g.y + height;
		});
		if (containing.length === 0) return null;
		containing.sort(
			(a, b) => (a.width ?? 300) * (a.height ?? 200) - (b.width ?? 300) * (b.height ?? 200)
		);
		return containing[0];
	}

	const nodeTypes: NodeTypes = {
		object: CustomObjectNode,
		text: CustomTextNode,
		customGroup: CustomGroupNode
	};
	const edgeTypes: EdgeTypes = { customView: CustomViewEdge };

	let paletteOpen = $state(true);
	let selectedNodeId = $state<string | null>(null);
	let selectedEdgeId = $state<string | null>(null);

	function resolveObjectData(view: CustomViewNodeRecord): CustomObjectNodeData {
		const ownImage = view.storage_path ? customViewNodeImageUrl(view.id) : null;
		const onResizeEnd = (bounds: CanvasNodeBounds) => handleNodeResizeEnd(view, bounds);

		if (view.kind === 'Library' && view.library_object_id) {
			const obj = (libraryObjectsQuery.data ?? []).find((o) => o.id === view.library_object_id);
			return {
				view,
				canvasDefaults,
				label: view.label || obj?.name || 'Object',
				imageUrl: ownImage ?? (obj?.storage_path ? libraryObjectImageUrl(obj.id) : null),
				icon: obj?.icon ?? null,
				onResizeEnd
			};
		}

		if (view.entity_type === 'Host') {
			const host = (hostsQuery.data?.items ?? []).find((h) => h.id === view.entity_id);
			const services = (servicesQuery.data?.items ?? []).filter((s) => s.host_id === host?.id);
			return {
				view,
				canvasDefaults,
				label: view.label || host?.name || 'Host',
				imageUrl:
					ownImage ??
					(host?.topology_icon_image_id ? hostImageContentUrl(host.topology_icon_image_id) : null),
				icon: 'server',
				headerText: host?.hostname || host?.manufacturer || null,
				services,
				onResizeEnd
			};
		}

		if (view.entity_type === 'Service') {
			const service = (servicesQuery.data?.items ?? []).find((s) => s.id === view.entity_id);
			return {
				view,
				canvasDefaults,
				label: view.label || service?.name || 'Service',
				imageUrl: ownImage,
				icon: 'layers',
				onResizeEnd
			};
		}

		return {
			view,
			canvasDefaults,
			label: view.label || 'Object',
			imageUrl: ownImage,
			icon: null,
			onResizeEnd
		};
	}

	/** Canvas-level typography every node inherits unless it overrides it. */
	let canvasDefaults = $derived({
		fontFamily: currentView?.default_font_family ?? null,
		fontSize: currentView?.default_font_size ?? null,
		textColor: currentView?.default_text_color ?? null,
		fontBold: currentView?.default_font_bold ?? null,
		fontItalic: currentView?.default_font_italic ?? null,
		fontUnderline: currentView?.default_font_underline ?? null,
		textAlign: currentView?.default_text_align ?? null
	});

	function toFlowNode(view: CustomViewNodeRecord): Node {
		if (view.kind === 'Group') {
			const data: CustomGroupNodeData = {
				view,
				canvasDefaults,
				onLabelChange: (label) => persistNodePatch(view, { label }),
				onResizeEnd: (bounds) => handleNodeResizeEnd(view, bounds)
			};
			return {
				id: view.id,
				type: 'customGroup',
				position: { x: view.x, y: view.y },
				width: view.width ?? 300,
				height: view.height ?? 200,
				data,
				selectable: true,
				selected: view.id === selectedNodeId,
				zIndex: 0
			};
		}

		if (view.kind === 'Text') {
			const data: CustomTextNodeData = {
				view,
				canvasDefaults,
				onTextChange: (text) => persistNodePatch(view, { text_content: text }),
				onResizeEnd: (bounds) => handleNodeResizeEnd(view, bounds),
				onAutoGrow: (bounds) => handleTextAutoGrow(view, bounds)
			};
			return {
				id: view.id,
				type: 'text',
				position: { x: view.x, y: view.y },
				width: view.width ?? 180,
				height: view.height ?? 80,
				data,
				selected: view.id === selectedNodeId,
				parentId: view.parent_node_id ?? undefined
			};
		}

		return {
			id: view.id,
			type: 'object',
			position: { x: view.x, y: view.y },
			width: view.width ?? 100,
			height: view.height ?? 100,
			data: resolveObjectData(view),
			selected: view.id === selectedNodeId,
			parentId: view.parent_node_id ?? undefined
		};
	}

	function toFlowEdge(edge: CustomViewEdgeRecord): Edge {
		const colorStyle = createColorHelper(edge.color ?? null);
		return {
			id: edge.id,
			source: edge.source_node_id,
			target: edge.target_node_id,
			sourceHandle: edge.source_handle ?? undefined,
			targetHandle: edge.target_handle ?? undefined,
			label: edge.label ?? undefined,
			animated: edge.is_dependency,
			selected: edge.id === selectedEdgeId,
			type: 'customView',
			data: {
				fontFamily: currentView?.default_font_family ?? null,
				fontSize: currentView?.default_font_size ?? 16,
				textColor: currentView?.default_text_color ?? null,
				fontBold: currentView?.default_font_bold ?? null,
				fontItalic: currentView?.default_font_italic ?? null,
				fontUnderline: currentView?.default_font_underline ?? null,
				textAlign: currentView?.default_text_align ?? null
			},
			style: `stroke: ${colorStyle.rgb};${edge.is_dependency ? 'stroke-dasharray: 6 4;' : ''}`
		};
	}

	// Group nodes must precede their children in xyflow's array for correct
	// parent/child resolution on first render.

	/**
	 * Fit the view to everything the user can see, not just the nodes.
	 *
	 * SvelteFlow's own fitView measures nodes only, so a connector label sitting
	 * outside every node box cannot be brought into view by it. Confirmed item 2
	 * requires those labels, and auto-grown free-text nodes, to be reachable
	 * through canvas navigation - and explicitly NOT by letting the page scroll.
	 * Labels are measured from the DOM because their size is whatever the browser
	 * rendered, then converted back into flow coordinates.
	 */
	function measureEdgeLabelRects(): Rect[] {
		if (typeof document === 'undefined') return [];

		const rects: Rect[] = [];
		for (const el of document.querySelectorAll('[data-edge-label-id]')) {
			const box = el.getBoundingClientRect();
			if (box.width === 0 && box.height === 0) continue;

			const topLeft = screenToFlowPosition({ x: box.left, y: box.top });
			const bottomRight = screenToFlowPosition({ x: box.right, y: box.bottom });
			rects.push({
				x: topLeft.x,
				y: topLeft.y,
				width: bottomRight.x - topLeft.x,
				height: bottomRight.y - topLeft.y
			});
		}
		return rects;
	}

	function fitCanvasContent() {
		const bounds = getCanvasContentBounds(flowNodes, measureEdgeLabelRects());
		if (!bounds) return;
		fitBounds(bounds, { padding: 0.1 });
	}

	let flowNodes = $derived.by<Node[]>(() => {
		const views = nodesQuery.data ?? [];
		const groups = views.filter((v) => v.kind === 'Group');
		const rest = views.filter((v) => v.kind !== 'Group');
		return [...groups, ...rest].map(toFlowNode);
	});

	let flowEdges = $derived.by<Edge[]>(() => (edgesQuery.data ?? []).map(toFlowEdge));

	async function persistNodePatch(
		view: CustomViewNodeRecord,
		patch: Partial<CustomViewNodeRecord>
	) {
		try {
			await updateNodeMutation.mutateAsync({ ...view, ...patch });
			return true;
		} catch (e) {
			pushError(e instanceof Error ? e.message : 'Failed to save node');
			return false;
		}
	}

	function defaultNodeSize(view: CustomViewNodeRecord): { width: number; height: number } {
		if (view.kind === 'Group') return { width: 300, height: 200 };
		if (view.kind === 'Text') return { width: 180, height: 80 };
		return { width: 100, height: 100 };
	}

	function persistedAbsolutePosition(
		view: CustomViewNodeRecord,
		views: CustomViewNodeRecord[]
	): { x: number; y: number } {
		const parent = view.parent_node_id
			? views.find((candidate) => candidate.id === view.parent_node_id)
			: null;
		return parent ? { x: parent.x + view.x, y: parent.y + view.y } : { x: view.x, y: view.y };
	}

	/** Snapshot the final rendered geometry once, at interaction completion. */
	function getCanvasGeometry(
		overrides: Map<string, Partial<CanvasNodeGeometry>> = new Map()
	): CanvasNodeGeometry[] {
		const views = nodesQuery.data ?? [];
		return views.map((view) => {
			const fallbackPosition = persistedAbsolutePosition(view, views);
			const fallbackSize = defaultNodeSize(view);
			const internal = getInternalNode(view.id);
			const override = overrides.get(view.id);
			return {
				id: view.id,
				kind: view.kind,
				parentNodeId: view.parent_node_id ?? null,
				x: override?.x ?? internal?.internals.positionAbsolute.x ?? fallbackPosition.x,
				y: override?.y ?? internal?.internals.positionAbsolute.y ?? fallbackPosition.y,
				width: override?.width ?? internal?.measured.width ?? view.width ?? fallbackSize.width,
				height: override?.height ?? internal?.measured.height ?? view.height ?? fallbackSize.height
			};
		});
	}

	function materializeMembershipPatches(
		patches: MembershipPatch[],
		extraPatches: Map<string, Partial<CustomViewNodeRecord>> = new Map()
	): CustomViewNodeRecord[] {
		const views = nodesQuery.data ?? [];
		const membershipById = new Map(patches.map((patch) => [patch.id, patch]));
		const ids = new Set([...membershipById.keys(), ...extraPatches.keys()]);
		return [...ids]
			.map((id) => {
				const view = views.find((candidate) => candidate.id === id);
				if (!view) return null;
				const membership = membershipById.get(id);
				return {
					...view,
					...(membership
						? {
								x: membership.x,
								y: membership.y,
								parent_node_id: membership.parentNodeId
							}
						: {}),
					...extraPatches.get(id)
				};
			})
			.filter((view): view is CustomViewNodeRecord => view !== null);
	}

	async function saveCompletedBoundsChange(
		changedNodeId: string,
		cause: CompletedBoundsChangeCause,
		geometry: CanvasNodeGeometry[],
		extraPatches: Map<string, Partial<CustomViewNodeRecord>> = new Map()
	) {
		const membership = reconcileCompletedBoundsChange(geometry, changedNodeId, cause);
		const updated = materializeMembershipPatches(membership, extraPatches);
		if (updated.length === 0) return;
		try {
			await saveLayoutMutation.mutateAsync({ viewId, nodes: updated, edges: [] });
		} catch (e) {
			pushError(e instanceof Error ? e.message : 'Failed to save layout');
		}
	}

	async function handleNodeResizeEnd(view: CustomViewNodeRecord, bounds: CanvasNodeBounds) {
		const currentGeometry = getCanvasGeometry();
		const parent = view.parent_node_id
			? (currentGeometry.find((node) => node.id === view.parent_node_id) ?? null)
			: null;
		const absoluteX = parent ? parent.x + bounds.x : bounds.x;
		const absoluteY = parent ? parent.y + bounds.y : bounds.y;
		const geometry = getCanvasGeometry(
			new Map([
				[view.id, { x: absoluteX, y: absoluteY, width: bounds.width, height: bounds.height }]
			])
		);
		const extra = new Map<string, Partial<CustomViewNodeRecord>>([
			[
				view.id,
				{
					width: Math.round(bounds.width),
					height: Math.round(bounds.height),
					...(view.kind === 'Group' ? { x: Math.round(absoluteX), y: Math.round(absoluteY) } : {})
				}
			]
		]);
		await saveCompletedBoundsChange(
			view.id,
			view.kind === 'Group' ? 'group-resize' : 'node-resize',
			geometry,
			extra
		);
	}

	async function handleTextAutoGrow(view: CustomViewNodeRecord, bounds: CanvasNodeBounds) {
		const currentGeometry = getCanvasGeometry();
		const rendered = currentGeometry.find((node) => node.id === view.id);
		if (!rendered) return;
		const geometry = getCanvasGeometry(
			new Map([[view.id, { width: bounds.width, height: bounds.height }]])
		);
		await saveCompletedBoundsChange(
			view.id,
			'text-auto-resize',
			geometry,
			new Map([[view.id, { width: Math.round(bounds.width), height: Math.round(bounds.height) }]])
		);
	}

	async function handleNodeDragStop(event: { targetNode: Node | null; nodes: Node[] }) {
		const moved = event.nodes.length > 0 ? event.nodes : event.targetNode ? [event.targetNode] : [];
		if (moved.length === 0) return;
		const views = nodesQuery.data ?? [];
		const geometry = getCanvasGeometry();
		const target = event.targetNode ?? moved[0];
		const targetView = views.find((view) => view.id === target.id);
		if (!targetView) return;

		if (targetView.kind === 'Group') {
			const rendered = geometry.find((node) => node.id === targetView.id);
			if (!rendered) return;
			await saveCompletedBoundsChange(
				targetView.id,
				'group-drag',
				geometry,
				new Map([
					[
						targetView.id,
						{ x: Math.round(rendered.x), y: Math.round(rendered.y), parent_node_id: null }
					]
				])
			);
			return;
		}

		const membership = moved.flatMap((node) => {
			const view = views.find((candidate) => candidate.id === node.id);
			return view && view.kind !== 'Group'
				? reconcileCompletedBoundsChange(geometry, node.id, 'node-drag')
				: [];
		});
		const updated = materializeMembershipPatches(membership);
		if (updated.length === 0) return;
		try {
			await saveLayoutMutation.mutateAsync({ viewId, nodes: updated, edges: [] });
		} catch (e) {
			pushError(e instanceof Error ? e.message : 'Failed to save layout');
		}
	}

	async function handleConnect(connection: {
		source: string;
		target: string;
		sourceHandle: string | null;
		targetHandle: string | null;
	}) {
		try {
			await createEdgeMutation.mutateAsync({
				view_id: viewId,
				network_id: networkId,
				source_node_id: connection.source,
				target_node_id: connection.target,
				source_handle: connection.sourceHandle,
				target_handle: connection.targetHandle,
				label: null,
				color: null,
				is_dependency: false,
				link_url: null
			});
		} catch (e) {
			pushError(e instanceof Error ? e.message : 'Failed to create edge');
		}
	}

	function handleSelectionChange(sel: { nodes: Node[]; edges: Edge[] }) {
		selectedNodeId = sel.nodes.length === 1 ? sel.nodes[0].id : null;
		selectedEdgeId = sel.edges.length === 1 ? sel.edges[0].id : null;
	}

	let selectedNode = $derived(
		selectedNodeId ? ((nodesQuery.data ?? []).find((n) => n.id === selectedNodeId) ?? null) : null
	);
	let selectedEdge = $derived(
		selectedEdgeId
			? ((edgesQuery.data ?? []).find((edge) => edge.id === selectedEdgeId) ?? null)
			: null
	);

	async function persistEdgePatch(
		edge: CustomViewEdgeRecord,
		patch: Partial<CustomViewEdgeRecord>
	) {
		try {
			await saveLayoutMutation.mutateAsync({ viewId, nodes: [], edges: [{ ...edge, ...patch }] });
		} catch (e) {
			pushError(e instanceof Error ? e.message : 'Failed to save join');
		}
	}

	async function uploadImageForSelected(file: File) {
		if (!selectedNodeId) return;
		try {
			await uploadNodeImageMutation.mutateAsync({ nodeId: selectedNodeId, file });
		} catch (e) {
			pushError(e instanceof Error ? e.message : 'Failed to upload image');
		}
	}

	async function handleDeleteSelected() {
		try {
			if (selectedNodeId) {
				await deleteNodeMutation.mutateAsync({ id: selectedNodeId, viewId });
				selectedNodeId = null;
			} else if (selectedEdgeId) {
				await deleteEdgeMutation.mutateAsync({ id: selectedEdgeId, viewId });
				selectedEdgeId = null;
			}
		} catch (e) {
			pushError(e instanceof Error ? e.message : 'Failed to delete');
		}
	}

	async function handleDeleteView() {
		if (!confirm(topology_customViewDeleteConfirm())) return;
		try {
			await deleteViewMutation.mutateAsync({ id: viewId, networkId });
			onClose();
		} catch (e) {
			pushError(e instanceof Error ? e.message : 'Failed to delete view');
		}
	}

	interface PaletteDropPayload {
		kind: 'entity' | 'library' | 'text' | 'group';
		entityType?: 'Host' | 'Service';
		entityId?: string;
		label?: string;
		libraryObjectId?: string;
	}

	function handleDragOver(event: DragEvent) {
		event.preventDefault();
	}

	async function handleDrop(event: DragEvent) {
		event.preventDefault();
		const raw = event.dataTransfer?.getData('application/x-scanopy-palette-item');
		if (!raw) return;
		let payload: PaletteDropPayload;
		try {
			payload = JSON.parse(raw);
		} catch {
			return;
		}

		const position = screenToFlowPosition({ x: event.clientX, y: event.clientY });
		const dropAbsoluteX = Math.round(position.x);
		const dropAbsoluteY = Math.round(position.y);

		// Group frames never auto-parent into another group (no nesting), so
		// their drop position is always absolute. Every other kind checks
		// whether it landed inside an existing group's bounds — using each
		// kind's default create size (matching toFlowNode's defaults) to find
		// the drop rect's center — and if so is created already parented,
		// with x/y stored relative to that group, exactly like a node dragged
		// into a group post-creation.
		function resolveDropPosition(width: number, height: number) {
			const enclosingGroup = findEnclosingGroup(
				dropAbsoluteX + width / 2,
				dropAbsoluteY + height / 2
			);
			if (!enclosingGroup) {
				return { x: dropAbsoluteX, y: dropAbsoluteY, parent_node_id: null as string | null };
			}
			return {
				x: dropAbsoluteX - enclosingGroup.x,
				y: dropAbsoluteY - enclosingGroup.y,
				parent_node_id: enclosingGroup.id as string | null
			};
		}

		try {
			if (payload.kind === 'entity') {
				await createNodeMutation.mutateAsync({
					view_id: viewId,
					network_id: networkId,
					...resolveDropPosition(100, 100),
					kind: 'Entity',
					entity_type: payload.entityType ?? null,
					entity_id: payload.entityId ?? null,
					label: payload.label ?? null,
					style: 'Image'
				});
			} else if (payload.kind === 'library') {
				await createNodeMutation.mutateAsync({
					view_id: viewId,
					network_id: networkId,
					...resolveDropPosition(100, 100),
					kind: 'Library',
					library_object_id: payload.libraryObjectId ?? null,
					label: payload.label ?? null,
					style: 'Image'
				});
			} else if (payload.kind === 'text') {
				await createNodeMutation.mutateAsync({
					view_id: viewId,
					network_id: networkId,
					...resolveDropPosition(180, 80),
					kind: 'Text',
					text_content: 'New note',
					color: 'Gray',
					font_family: currentView?.default_font_family ?? null,
					font_size: currentView?.default_font_size ?? 16
				});
			} else if (payload.kind === 'group') {
				await createNodeMutation.mutateAsync({
					view_id: viewId,
					network_id: networkId,
					x: dropAbsoluteX,
					y: dropAbsoluteY,
					kind: 'Group',
					label: 'New group',
					color: 'Blue',
					corner_style: 'Rounded',
					width: 300,
					height: 200
				});
			}
		} catch (e) {
			pushError(e instanceof Error ? e.message : 'Failed to add object');
		}
	}
</script>

<div class="custom-view-canvas relative flex h-full min-h-0 w-full">
	{#if paletteOpen}
		<CustomViewPalette
			hosts={identifiedHosts}
			services={servicesQuery.data?.items ?? []}
			libraryObjects={libraryObjectsQuery.data ?? []}
			ipAddresses={ipAddressesQuery.data ?? []}
			ports={portsQuery.data ?? []}
		/>
	{/if}

	<div
		class="relative flex-1"
		style:background-color={currentView?.background_color
			? createColorHelper(currentView.background_color).rgb
			: undefined}
		ondragover={handleDragOver}
		ondrop={handleDrop}
		role="application"
	>
		{#if currentView}
			<CanvasControlPanel
				view={currentView}
				onUpdate={(patch) => updateViewMutation.mutateAsync({ ...currentView, ...patch })}
				onTogglePalette={() => (paletteOpen = !paletteOpen)}
				onDelete={handleDeleteView}
				{onClose}
			/>
		{/if}

		<SvelteFlow
			nodes={flowNodes}
			edges={flowEdges}
			{nodeTypes}
			{edgeTypes}
			oninit={() => requestAnimationFrame(fitCanvasContent)}
			minZoom={0.1}
			snapGrid={(currentView?.snap_to_grid ?? true)
				? [currentView?.grid_size ?? 10, currentView?.grid_size ?? 10]
				: [1, 1]}
			nodesDraggable={true}
			nodesConnectable={true}
			elementsSelectable={true}
			connectionMode={ConnectionMode.Loose}
			onnodedragstop={handleNodeDragStop}
			onconnect={handleConnect}
			onselectionchange={handleSelectionChange}
		>
			{#if currentView?.show_grid ?? true}
				<Background variant={BackgroundVariant.Dots} gap={40} size={1} />
			{/if}
			<MiniMap position="bottom-left" />
		</SvelteFlow>
	</div>

	{#if selectedNode}
		<CustomViewNodeInspector
			node={selectedNode}
			{canvasDefaults}
			libraryObjects={libraryObjectsQuery.data ?? []}
			onUpdate={(patch) => persistNodePatch(selectedNode!, patch)}
			onUploadImage={(file) => uploadImageForSelected(file)}
			onDelete={handleDeleteSelected}
		/>
	{:else if selectedEdge}
		<div
			class="absolute right-2 top-2 z-10 w-64 space-y-3 rounded-md bg-white p-3 shadow dark:bg-gray-900"
		>
			<strong class="text-sm">Join settings</strong>
			<label class="block text-xs font-medium"
				>Connection text
				<input
					class="input-field mt-1 w-full"
					value={selectedEdge.label ?? ''}
					onchange={(event) =>
						persistEdgePatch(selectedEdge!, {
							label: (event.target as HTMLInputElement).value || null
						})}
				/>
			</label>
			<label class="flex items-center gap-2 text-xs font-medium"
				><input
					type="checkbox"
					checked={selectedEdge.is_dependency}
					onchange={(event) =>
						persistEdgePatch(selectedEdge!, {
							is_dependency: (event.target as HTMLInputElement).checked
						})}
				/> Dependency</label
			>
			<label class="block text-xs font-medium"
				>Link URL
				<input
					class="input-field mt-1 w-full"
					type="url"
					value={selectedEdge.link_url ?? ''}
					placeholder="https://…"
					onchange={(event) =>
						persistEdgePatch(selectedEdge!, {
							link_url: (event.target as HTMLInputElement).value || null
						})}
				/>
			</label>
			{#if getSafeCanvasLink(selectedEdge.link_url)}
				<button
					class="btn-secondary w-full text-xs"
					onclick={() =>
						window.open(
							getSafeCanvasLink(selectedEdge!.link_url)!,
							'_blank',
							'noopener,noreferrer'
						)}>Open link</button
				>
			{/if}
			<button class="btn-danger" onclick={handleDeleteSelected}>Delete edge</button>
		</div>
	{/if}
</div>

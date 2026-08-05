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
		type NodeTypes
	} from '@xyflow/svelte';
	import '@xyflow/svelte/dist/style.css';
	import { X, PenTool, Blocks, Trash2 } from 'lucide-svelte';
	import CustomObjectNode from './CustomObjectNode.svelte';
	import CustomTextNode from './CustomTextNode.svelte';
	import CustomGroupNode from './CustomGroupNode.svelte';
	import CustomViewPalette from './CustomViewPalette.svelte';
	import CustomViewNodeInspector from './CustomViewNodeInspector.svelte';
	import type { CustomObjectNodeData, CustomTextNodeData, CustomGroupNodeData } from './types';
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
		useLibraryObjectsQuery,
		useUploadCustomViewNodeImageMutation,
		customViewNodeImageUrl,
		libraryObjectImageUrl,
		type CustomViewNode as CustomViewNodeRecord,
		type CustomViewEdge as CustomViewEdgeRecord
	} from '$lib/features/custom-topology-views/queries';
	import { useHostsQuery } from '$lib/features/hosts/queries';
	import { useServicesQuery } from '$lib/features/services/queries';
	import { hostImageContentUrl } from '$lib/features/host-images/queries';
	import { createColorHelper } from '$lib/shared/utils/styling';
	import { getSafeCanvasLink } from './custom-view-model';
	import { pushError } from '$lib/shared/stores/feedback';
	import {
		common_close,
		topology_customViewDeleteConfirm,
		topology_customViewDeleteView,
		topology_customViewTogglePalette
	} from '$lib/paraglide/messages';

	interface Props {
		viewId: string;
		networkId: string;
		viewName: string;
		onClose: () => void;
	}

	let { viewId, networkId, viewName, onClose }: Props = $props();

	const nodesQuery = useCustomViewNodesQuery(() => viewId);
	const edgesQuery = useCustomViewEdgesQuery(() => viewId);
	const hostsQuery = useHostsQuery(() => ({ network_id: networkId, limit: 0 }));
	const servicesQuery = useServicesQuery(() => ({ network_id: networkId, limit: 0 }));
	const libraryObjectsQuery = useLibraryObjectsQuery();

	const createNodeMutation = useCreateCustomViewNodeMutation();
	const updateNodeMutation = useUpdateCustomViewNodeMutation();
	const deleteNodeMutation = useDeleteCustomViewNodeMutation();
	const createEdgeMutation = useCreateCustomViewEdgeMutation();
	const deleteEdgeMutation = useDeleteCustomViewEdgeMutation();
	const saveLayoutMutation = useSaveCustomTopologyViewLayoutMutation();
	const uploadNodeImageMutation = useUploadCustomViewNodeImageMutation();
	const deleteViewMutation = useDeleteCustomTopologyViewMutation();

	const { screenToFlowPosition } = useSvelteFlow();

	const nodeTypes: NodeTypes = {
		object: CustomObjectNode,
		text: CustomTextNode,
		customGroup: CustomGroupNode
	};

	let paletteOpen = $state(true);
	let selectedNodeId = $state<string | null>(null);
	let selectedEdgeId = $state<string | null>(null);

	function resolveObjectData(view: CustomViewNodeRecord): CustomObjectNodeData {
		const ownImage = view.storage_path ? customViewNodeImageUrl(view.id) : null;
		const onResizeEnd = (width: number, height: number) =>
			persistNodePatch(view, { width, height });

		if (view.kind === 'Library' && view.library_object_id) {
			const obj = (libraryObjectsQuery.data ?? []).find((o) => o.id === view.library_object_id);
			return {
				view,
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
				label: view.label || service?.name || 'Service',
				imageUrl: ownImage,
				icon: 'layers',
				onResizeEnd
			};
		}

		return { view, label: view.label || 'Object', imageUrl: ownImage, icon: null, onResizeEnd };
	}

	function toFlowNode(view: CustomViewNodeRecord): Node {
		if (view.kind === 'Group') {
			const data: CustomGroupNodeData = {
				view,
				onLabelChange: (label) => persistNodePatch(view, { label }),
				onResizeEnd: (width, height) => persistNodePatch(view, { width, height })
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
				onTextChange: (text) => persistNodePatch(view, { text_content: text }),
				onResizeEnd: (width, height) => persistNodePatch(view, { width, height })
			};
			return {
				id: view.id,
				type: 'text',
				position: { x: view.x, y: view.y },
				width: view.width ?? 180,
				height: view.height ?? 80,
				data,
				selected: view.id === selectedNodeId,
				parentId: view.parent_node_id ?? undefined,
				extent: view.parent_node_id ? 'parent' : undefined
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
			parentId: view.parent_node_id ?? undefined,
			extent: view.parent_node_id ? 'parent' : undefined
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
			style: `stroke: ${colorStyle.rgb};${edge.is_dependency ? 'stroke-dasharray: 6 4;' : ''}`
		};
	}

	// Group nodes must precede their children in xyflow's array for correct
	// parent/child resolution on first render.
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
		} catch (e) {
			pushError(e instanceof Error ? e.message : 'Failed to save node');
		}
	}

	async function handleNodeDragStop(event: { targetNode: Node | null; nodes: Node[] }) {
		const moved = event.nodes.length > 0 ? event.nodes : event.targetNode ? [event.targetNode] : [];
		if (moved.length === 0) return;
		const views = nodesQuery.data ?? [];
		const updated = moved
			.map((n) => {
				const view = views.find((v) => v.id === n.id);
				if (!view) return null;
				return { ...view, x: Math.round(n.position.x), y: Math.round(n.position.y) };
			})
			.filter((v): v is CustomViewNodeRecord => v !== null);
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
		const base = {
			view_id: viewId,
			network_id: networkId,
			x: Math.round(position.x),
			y: Math.round(position.y)
		};

		try {
			if (payload.kind === 'entity') {
				await createNodeMutation.mutateAsync({
					...base,
					kind: 'Entity',
					entity_type: payload.entityType ?? null,
					entity_id: payload.entityId ?? null,
					label: payload.label ?? null,
					style: 'Image'
				});
			} else if (payload.kind === 'library') {
				await createNodeMutation.mutateAsync({
					...base,
					kind: 'Library',
					library_object_id: payload.libraryObjectId ?? null,
					label: payload.label ?? null,
					style: 'Image'
				});
			} else if (payload.kind === 'text') {
				await createNodeMutation.mutateAsync({
					...base,
					kind: 'Text',
					text_content: 'New note',
					color: 'Gray',
					font_family: 'Sans',
					font_size: 16
				});
			} else if (payload.kind === 'group') {
				await createNodeMutation.mutateAsync({
					...base,
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

<div class="custom-view-canvas relative flex h-full min-h-[600px] w-full">
	{#if paletteOpen}
		<CustomViewPalette
			hosts={hostsQuery.data?.items ?? []}
			services={servicesQuery.data?.items ?? []}
			libraryObjects={libraryObjectsQuery.data ?? []}
		/>
	{/if}

	<div class="relative flex-1" ondragover={handleDragOver} ondrop={handleDrop} role="application">
		<div
			class="absolute left-2 top-2 z-10 flex items-center gap-2 rounded-md bg-white/90 px-3 py-1.5 shadow dark:bg-gray-900/90"
		>
			<PenTool class="h-4 w-4 text-pink-500" />
			<span class="text-sm font-medium">{viewName}</span>
			<button
				class="btn-icon ml-2"
				title={topology_customViewTogglePalette()}
				onclick={() => (paletteOpen = !paletteOpen)}
			>
				<Blocks class="h-4 w-4" />
			</button>
			<button
				class="btn-icon text-red-500"
				title={topology_customViewDeleteView()}
				onclick={handleDeleteView}
			>
				<Trash2 class="h-4 w-4" />
			</button>
			<button class="btn-icon" title={common_close()} onclick={onClose}>
				<X class="h-4 w-4" />
			</button>
		</div>

		<SvelteFlow
			nodes={flowNodes}
			edges={flowEdges}
			{nodeTypes}
			fitView={true}
			minZoom={0.1}
			snapGrid={[10, 10]}
			nodesDraggable={true}
			nodesConnectable={true}
			elementsSelectable={true}
			connectionMode={ConnectionMode.Loose}
			onnodedragstop={handleNodeDragStop}
			onconnect={handleConnect}
			onselectionchange={handleSelectionChange}
		>
			<Background variant={BackgroundVariant.Dots} gap={40} size={1} />
			<MiniMap position="bottom-left" />
		</SvelteFlow>
	</div>

	{#if selectedNode}
		<CustomViewNodeInspector
			node={selectedNode}
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

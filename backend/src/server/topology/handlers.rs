use crate::server::openapi::tags as api_tags;
use crate::server::shared::extractors::Query;
use crate::server::shared::storage::traits::Entity;
use crate::server::{
    auth::middleware::{
        auth::AuthenticatedEntity,
        permissions::{Authorized, IsUser, Member, Viewer},
    },
    config::AppState,
    shared::{
        events::{
            traits::{Event as BusEvent, OrgScope},
            types::{OnboardingOperation, OnboardingOperationDiscriminants},
        },
        handlers::{
            query::{FilterQueryExtractor, NetworkFilterQuery},
            traits::{CrudHandlers, update_handler},
        },
        services::traits::CrudService,
        storage::filter::StorableFilter,
        types::api::{
            ApiError, ApiErrorResponse, ApiResponse, ApiResult, EmptyApiResponse,
            PaginatedApiResponse, ValidationError,
        },
    },
    topology::types::{
        api::{TopologyData, TopologyNodePosition},
        base::{Topology, TopologyNodePositionUpdate},
        layout::Ixy,
        nodes::{ContainerType, Node, NodeType},
        views::TopologyView,
    },
};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, header},
    response::{
        IntoResponse, Json, Sse,
        sse::{Event, KeepAlive},
    },
    routing::get,
};
use futures::{Stream, stream};
use serde::Deserialize;
use serde_json::json;
use std::{convert::Infallible, sync::Arc};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

// Generated handlers for generic CRUD operations
mod generated {
    use super::*;
    crate::crud_get_by_id_handler!(Topology);
    crate::crud_export_csv_handler!(Topology);
}

/// Topology endpoints are internal-only (hidden from public docs).
///
/// A topology row now holds only the user's grouping `options` (one live row
/// per network). The per-view graph is built on request and returned on the
/// `/data` bundle; `get_all` / `get_by_id` list the slim rows, `update_topology`
/// persists `options`, alongside exports, manual node positions, and a single
/// SSE channel.
///
/// The live row is auto-created at network creation; snapshots build their
/// graph on request from closed copies and never receive mutable position
/// overrides. Node resize and edge-handle mutations remain unsupported.
pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all_topologies))
        .routes(routes!(get_topology_data))
        .routes(routes!(generated::get_by_id, update_topology))
        .routes(routes!(generated::export_csv))
        .routes(routes!(export_mermaid))
        .routes(routes!(export_confluence))
        .routes(routes!(update_node_position))
        .routes(routes!(reset_node_positions))
        // SSE endpoint (not well-supported by OpenAPI)
        .route("/stream", get(live_topology_updates_stream))
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct TopologyDataQuery {
    /// Network to read entities for. Required.
    pub network_id: Uuid,
    /// When set, returns the entity set as it was when this snapshot was taken.
    /// When omitted, returns live entities.
    #[serde(default)]
    pub snapshot_id: Option<Uuid>,
    /// When `true`, records the `FirstTopologyRebuild` onboarding milestone (the user has
    /// viewed their topology). Only the frontend's explicit on-tab view sets this — the
    /// background topology-data query never does — so the milestone never fires from other
    /// tabs. One-time per org (guarded below + subscriber dedup).
    #[serde(default)]
    pub mark_viewed: Option<bool>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct TopologyExportQuery {
    /// View to export. Defaults to the default view when omitted.
    #[serde(default)]
    pub view: TopologyView,
}

/// Unified entity-set endpoint for the topology view.
///
/// `?snapshot_id=<id>` resolves to the snapshot's `taken_at` and returns the
/// as-of-T entity set; otherwise returns live entities. The frontend
/// `TopologyTab` is the sole intended consumer.
#[utoipa::path(
    get,
    path = "/data",
    tags = [Topology::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    params(TopologyDataQuery),
    responses(
        (status = 200, description = "Topology entity bundle", body = ApiResponse<TopologyData>),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
        (status = 404, description = "Snapshot not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_topology_data(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Query(params): Query<TopologyDataQuery>,
) -> ApiResult<Json<ApiResponse<TopologyData>>> {
    let network_ids = auth.network_ids();
    if !network_ids.contains(&params.network_id) {
        return Err(ApiError::forbidden("You don't have access to this network"));
    }

    // Snapshot path: verify the snapshot belongs to the requested network
    // before returning its closed copies.
    if let Some(snapshot_id) = params.snapshot_id {
        let snapshot = state
            .services
            .snapshot_service
            .get_by_id(&snapshot_id)
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?
            .ok_or_else(|| ApiError::not_found("Snapshot not found".to_string()))?;
        if snapshot.base.network_id != params.network_id {
            return Err(ApiError::forbidden(
                "Snapshot belongs to a different network",
            ));
        }
    }

    // Build the per-view graph on request from entities + the network's
    // grouping options (live or snapshot entity set). The frontend selects the
    // active view's slice client-side and runs ELK each render.
    let data = state
        .services
        .topology_service
        .get_topology_render_data(params.network_id, params.snapshot_id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    // Onboarding: mark the topology "viewed" only when the frontend explicitly requests it
    // (`mark_viewed=true`), which it does solely while the topology tab is focused. The
    // background topology-data query never sets the flag, so the milestone can't fire from
    // other tabs. Still gated to a live view with hosts and a completed discovery, one-time
    // per org (the not_onboarded guard here + the subscriber's dedup).
    if params.mark_viewed.unwrap_or(false)
        && params.snapshot_id.is_none()
        && !data.hosts.is_empty()
        && let Ok(Some(network)) = state
            .services
            .network_service
            .get_by_id(&params.network_id)
            .await
        && let Ok(Some(org)) = state
            .services
            .organization_service
            .get_by_id(&network.base.organization_id)
            .await
        && org.has_onboarded(&OnboardingOperationDiscriminants::FirstDiscoveryCompleted)
        && org.not_onboarded(&OnboardingOperationDiscriminants::FirstTopologyRebuild)
    {
        let _ = state
            .services
            .event_bus
            .publish(BusEvent::new(
                OrgScope {
                    organization_id: org.id,
                },
                OnboardingOperation::FirstTopologyRebuild,
                AuthenticatedEntity::System,
            ))
            .await;
    }

    Ok(Json(ApiResponse::success(data)))
}

#[utoipa::path(
    put,
    path = "/{id}",
    tags = [Topology::ENTITY_NAME_PLURAL],
    params(("id" = Uuid, Path, description = "Topology ID")),
    responses(
        (status = 200, description = "Topology updated", body = ApiResponse<Topology>),
        (status = 404, description = "Topology not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn update_topology(
    state: State<Arc<AppState>>,
    auth: Authorized<Member>,
    id: Path<Uuid>,
    topology: Json<Topology>,
) -> ApiResult<Json<ApiResponse<Topology>>> {
    // `snapshot_id` is immutable post-insert; `Storable::preserve_immutable_fields`
    // restores it from the existing row before the UPDATE statement runs, so
    // any client-supplied value here is silently ignored. Layout changes
    // (nodes/edges/options) are the only mutations this handler accepts.
    update_handler::<Topology>(state, auth, id, topology).await
}

/// Get all topologies for the authenticated user's networks.
///
/// Returns both live-view rows (`snapshot_id IS NULL`) and snapshot-pinned
/// rows. The frontend renders the live one by default and renders snapshot
/// rows when the user picks one from the snapshots dropdown.
#[utoipa::path(
    get,
    path = "",
    tags = [Topology::ENTITY_NAME_PLURAL],
    params(NetworkFilterQuery),
    responses(
        (status = 200, description = "List of topologies", body = PaginatedApiResponse<Topology>),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn get_all_topologies(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    query: Query<NetworkFilterQuery>,
) -> ApiResult<Json<PaginatedApiResponse<Topology>>> {
    let network_ids = auth.network_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;

    let base_filter = StorableFilter::<Topology>::new_from_network_ids(&network_ids);
    let filter = query.apply_to_filter(base_filter, &network_ids, organization_id);
    let pagination = query.pagination();
    let filter = pagination.apply_to_filter(filter);

    let service = Topology::get_service(&state);
    let result = service.get_paginated(filter).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to fetch topologies");
        ApiError::internal_error(&e.to_string())
    })?;

    let limit = pagination.effective_limit().unwrap_or(0);
    let offset = pagination.effective_offset();

    Ok(Json(PaginatedApiResponse::success(
        result.items,
        result.total_count,
        limit,
        offset,
    )))
}

const MAX_LAYOUT_COORDINATE: isize = 1_000_000;

fn validate_layout_position(position: Ixy) -> ApiResult<()> {
    if !(-MAX_LAYOUT_COORDINATE..=MAX_LAYOUT_COORDINATE).contains(&position.x)
        || !(-MAX_LAYOUT_COORDINATE..=MAX_LAYOUT_COORDINATE).contains(&position.y)
    {
        return Err(ApiError::bad_request(&format!(
            "Node coordinates must be between -{} and {}",
            MAX_LAYOUT_COORDINATE, MAX_LAYOUT_COORDINATE
        )));
    }
    Ok(())
}

/// Return the server-derived parent for a node the user may manually move.
/// Elements are movable in every view. Host containers are also movable so a
/// complete host card can be arranged in host-centric views. Other generated
/// containers remain layout-owned.
fn movable_node_parent(node: &Node) -> Option<Option<Uuid>> {
    match &node.node_type {
        NodeType::Element { container_id, .. } => Some(Some(*container_id)),
        NodeType::Container {
            container_type: ContainerType::Host,
            parent_container_id,
            ..
        } => Some(*parent_container_id),
        NodeType::Container { .. } => None,
    }
}

fn layout_storage_error(error: anyhow::Error) -> ApiError {
    if let Some(validation) = error.downcast_ref::<ValidationError>() {
        ApiError::bad_request(&validation.0)
    } else {
        tracing::error!(error = %error, "Failed to persist topology node position");
        ApiError::internal_error(&error.to_string())
    }
}

/// Save one manual node position in one live topology view.
#[utoipa::path(
    post,
    path = "/{id}/node-position",
    tags = [Topology::ENTITY_NAME_PLURAL, "internal"],
    params(("id" = Uuid, Path, description = "Topology ID")),
    request_body = TopologyNodePositionUpdate,
    responses(
        (status = 200, description = "Node position saved", body = ApiResponse<TopologyNodePosition>),
        (status = 400, description = "Invalid node or coordinates", body = ApiErrorResponse),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
        (status = 404, description = "Topology or node not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn update_node_position(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
    Json(request): Json<TopologyNodePositionUpdate>,
) -> ApiResult<Json<ApiResponse<TopologyNodePosition>>> {
    validate_layout_position(request.position)?;

    let service = Topology::get_service(&state);
    let topology = service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Topology {} not found", id)))?;

    // Authorize against the persisted topology's actual network; the request
    // body deliberately carries no client-supplied network id.
    if !auth.network_ids().contains(&topology.base.network_id) {
        return Err(ApiError::forbidden(
            "You don't have access to this topology's network",
        ));
    }

    // Rebuild from live data before accepting the write. This prevents stale,
    // fabricated, or non-movable generated node IDs from accumulating rows.
    let render_data = service
        .get_topology_render_data(topology.base.network_id, None)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    let node = render_data
        .nodes
        .get(&request.view)
        .and_then(|nodes| nodes.iter().find(|node| node.id == request.node_id))
        .ok_or_else(|| {
            ApiError::not_found(format!(
                "Node {} not found in {:?} view",
                request.node_id, request.view
            ))
        })?;
    let parent_node_id = movable_node_parent(node).ok_or_else(|| {
        ApiError::bad_request("Only topology elements and host containers can be moved")
    })?;

    let saved = service
        .upsert_layout_override(
            topology.id,
            topology.base.network_id,
            request.view,
            request.node_id,
            parent_node_id,
            request.position,
        )
        .await
        .map_err(layout_storage_error)?;

    Ok(Json(ApiResponse::success(saved)))
}

/// Reset all manual positions for one view without changing other views.
#[utoipa::path(
    delete,
    path = "/{id}/node-positions/{view}",
    tags = [Topology::ENTITY_NAME_PLURAL, "internal"],
    params(
        ("id" = Uuid, Path, description = "Topology ID"),
        ("view" = TopologyView, Path, description = "Topology view")
    ),
    responses(
        (status = 200, description = "View positions reset", body = EmptyApiResponse),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
        (status = 404, description = "Topology not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn reset_node_positions(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path((id, view)): Path<(Uuid, TopologyView)>,
) -> ApiResult<Json<EmptyApiResponse>> {
    let service = Topology::get_service(&state);
    let topology = service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Topology {} not found", id)))?;

    if !auth.network_ids().contains(&topology.base.network_id) {
        return Err(ApiError::forbidden(
            "You don't have access to this topology's network",
        ));
    }

    service
        .reset_layout_overrides(topology.id, topology.base.network_id, view)
        .await
        .map_err(layout_storage_error)?;
    Ok(Json(ApiResponse::success(())))
}

// === DISABLED: legacy graph mutation endpoints ===
// Node resize and edge-handle reconnect remain intentionally disabled. The
// manual-position endpoint above persists only x/y placement in the dedicated
// override table; it does not revive graph, size, or edge persistence.
/*
/// Update a single node's position
#[utoipa::path(
    post,
    path = "/{id}/node-position",
    tags = [Topology::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    params(("id" = Uuid, Path, description = "Topology ID")),
    request_body = TopologyNodePositionUpdate,
    responses(
        (status = 200, description = "Node position updated", body = EmptyApiResponse),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
        (status = 404, description = "Topology or node not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn update_node_position(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
    Json(request): Json<TopologyNodePositionUpdate>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let network_ids = auth.network_ids();

    if !network_ids.contains(&request.network_id) {
        return Err(ApiError::forbidden(
            "You don't have access to this topology's network",
        ));
    }

    let service = Topology::get_service(&state);

    let mut topology = service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Topology {} not found", id)))?;

    let node = topology
        .base
        .nodes
        .get_mut(&request.view)
        .and_then(|nodes| nodes.iter_mut().find(|n| n.id == request.node_id))
        .ok_or_else(|| {
            ApiError::not_found(format!(
                "Node {} not found in {:?} view",
                request.node_id, request.view
            ))
        })?;

    node.position = request.position;

    service.update(&mut topology, auth.into_entity()).await?;

    Ok(Json(ApiResponse::success(())))
}

/// Update an edge's handles
#[utoipa::path(
    post,
    path = "/{id}/edge-handles",
    tags = [Topology::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    params(("id" = Uuid, Path, description = "Topology ID")),
    request_body = TopologyEdgeHandleUpdate,
    responses(
        (status = 200, description = "Edge handles updated", body = EmptyApiResponse),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
        (status = 404, description = "Topology or edge not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn update_edge_handles(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
    Json(request): Json<TopologyEdgeHandleUpdate>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let network_ids = auth.network_ids();

    if !network_ids.contains(&request.network_id) {
        return Err(ApiError::forbidden(
            "You don't have access to this topology's network",
        ));
    }

    let service = Topology::get_service(&state);

    let mut topology = service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Topology {} not found", id)))?;

    let edge = topology
        .base
        .edges
        .get_mut(&request.view)
        .and_then(|edges| edges.iter_mut().find(|e| e.id == request.edge_id))
        .ok_or_else(|| {
            ApiError::not_found(format!(
                "Edge {} not found in {:?} view",
                request.edge_id, request.view
            ))
        })?;

    edge.source_handle = request.source_handle;
    edge.target_handle = request.target_handle;

    service.update(&mut topology, auth.into_entity()).await?;

    Ok(Json(ApiResponse::success(())))
}

/// Update a node's size and position
#[utoipa::path(
    post,
    path = "/{id}/node-resize",
    tags = [Topology::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    params(("id" = Uuid, Path, description = "Topology ID")),
    request_body = TopologyNodeResizeUpdate,
    responses(
        (status = 200, description = "Node resized", body = EmptyApiResponse),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
        (status = 404, description = "Topology or node not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn update_node_resize(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
    Json(request): Json<TopologyNodeResizeUpdate>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let network_ids = auth.network_ids();

    if !network_ids.contains(&request.network_id) {
        return Err(ApiError::forbidden(
            "You don't have access to this topology's network",
        ));
    }

    let service = Topology::get_service(&state);

    let mut topology = service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Topology {} not found", id)))?;

    let node = topology
        .base
        .nodes
        .get_mut(&request.view)
        .and_then(|nodes| nodes.iter_mut().find(|n| n.id == request.node_id))
        .ok_or_else(|| {
            ApiError::not_found(format!(
                "Node {} not found in {:?} view",
                request.node_id, request.view
            ))
        })?;

    node.size = request.size;
    node.position = request.position;

    service.update(&mut topology, auth.into_entity()).await?;

    Ok(Json(ApiResponse::success(())))
}
*/

/// Export topology as Mermaid flowchart
#[utoipa::path(
    get,
    path = "/{id}/export/mermaid",
    tags = [Topology::ENTITY_NAME_PLURAL],
    params(("id" = Uuid, Path, description = "Topology ID"), TopologyExportQuery),
    responses(
        (status = 200, description = "Mermaid flowchart export", content_type = "text/plain", body = String),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
        (status = 404, description = "Topology not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn export_mermaid(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Path(id): Path<Uuid>,
    query: Query<TopologyExportQuery>,
) -> ApiResult<impl IntoResponse> {
    let network_ids = auth.network_ids();

    let service = Topology::get_service(&state);
    let topology = service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Topology {} not found", id)))?;

    if !network_ids.contains(&topology.base.network_id) {
        return Err(ApiError::forbidden(
            "You don't have access to this topology",
        ));
    }

    // Build the graph on request and export the requested view's slice.
    let data = service
        .get_topology_render_data(topology.base.network_id, None)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    let nodes = data
        .nodes
        .get(&query.view)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let edges = data
        .edges
        .get(&query.view)
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    let content = crate::server::topology::types::export::topology_to_mermaid(nodes, edges, &data);

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"topology.mmd\""),
    );

    Ok((headers, Body::from(content)))
}

/// Export topology as Confluence wiki markup
#[utoipa::path(
    get,
    path = "/{id}/export/confluence",
    tags = [Topology::ENTITY_NAME_PLURAL],
    params(("id" = Uuid, Path, description = "Topology ID"), TopologyExportQuery),
    responses(
        (status = 200, description = "Confluence wiki markup export", content_type = "text/plain", body = String),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
        (status = 404, description = "Topology not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn export_confluence(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Path(id): Path<Uuid>,
    query: Query<TopologyExportQuery>,
) -> ApiResult<impl IntoResponse> {
    let network_ids = auth.network_ids();

    let service = Topology::get_service(&state);
    let topology = service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Topology {} not found", id)))?;

    if !network_ids.contains(&topology.base.network_id) {
        return Err(ApiError::forbidden(
            "You don't have access to this topology",
        ));
    }

    // Build the graph on request and export the requested view's slice.
    let data = service
        .get_topology_render_data(topology.base.network_id, None)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    let nodes = data
        .nodes
        .get(&query.view)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let edges = data
        .edges
        .get(&query.view)
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    let content =
        crate::server::topology::types::export::topology_to_confluence(nodes, edges, &data);

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"topology.txt\""),
    );

    Ok((headers, Body::from(content)))
}

/// SSE stream of live-topology updates: emits `{ "network_id": "<uuid>" }`
/// on every change to a network's live entity set. Frontends invalidate
/// their topology query and refetch on receipt.
async fn live_topology_updates_stream(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsUser>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state
        .services
        .topology_service
        .subscribe_live_topology_updates();

    let allowed_networks = auth.network_ids();

    let stream = stream::unfold(rx, move |mut rx| {
        let allowed = allowed_networks.clone();
        async move {
            loop {
                match rx.recv().await {
                    Ok(network_id) => {
                        if allowed.contains(&network_id) {
                            let payload = json!({ "network_id": network_id }).to_string();
                            return Some((Ok(Event::default().data(payload)), rx));
                        }
                    }
                    Err(_) => return None,
                }
            }
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

#[cfg(test)]
mod layout_position_tests {
    use super::*;
    use crate::server::topology::types::{layout::Uxy, nodes::ElementEntityType};

    #[test]
    fn coordinate_bounds_are_signed_and_inclusive() {
        assert!(
            validate_layout_position(Ixy {
                x: -MAX_LAYOUT_COORDINATE,
                y: MAX_LAYOUT_COORDINATE,
            })
            .is_ok()
        );
        assert!(
            validate_layout_position(Ixy {
                x: MAX_LAYOUT_COORDINATE + 1,
                y: 0,
            })
            .is_err()
        );
        assert!(
            validate_layout_position(Ixy {
                x: 0,
                y: -MAX_LAYOUT_COORDINATE - 1,
            })
            .is_err()
        );
    }

    #[test]
    fn elements_use_their_freshly_derived_container_as_parent() {
        let container_id = Uuid::new_v4();
        let node = Node::element(
            Uuid::new_v4(),
            container_id,
            Uuid::new_v4(),
            ElementEntityType::Service {},
        );
        assert_eq!(movable_node_parent(&node), Some(Some(container_id)));
    }

    #[test]
    fn host_containers_are_movable_but_other_containers_are_not() {
        let parent_id = Uuid::new_v4();
        let container = |container_type| Node {
            id: Uuid::new_v4(),
            node_type: NodeType::Container {
                container_type,
                parent_container_id: Some(parent_id),
                entity_id: Some(Uuid::new_v4()),
                icon: None,
                color: None,
                associated_service_definition: None,
                element_rule_id: None,
                will_accept_edges: false,
            },
            position: Ixy::default(),
            size: Uxy::default(),
            header: None,
        };

        assert_eq!(
            movable_node_parent(&container(ContainerType::Host)),
            Some(Some(parent_id))
        );
        assert_eq!(movable_node_parent(&container(ContainerType::Subnet)), None);
    }
}

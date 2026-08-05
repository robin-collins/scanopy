use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::server::auth::middleware::permissions::{Authorized, Member};
use crate::server::config::AppState;
use crate::server::custom_topology_views::{
    r#impl::base::CustomTopologyView, service::CustomTopologyViewService,
};
use crate::server::custom_view_edges::r#impl::base::CustomViewEdge;
use crate::server::custom_view_nodes::r#impl::base::CustomViewNode;
use crate::server::shared::handlers::query::NetworkFilterQuery;
use crate::server::shared::handlers::traits::CrudHandlers;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::traits::Entity;
use crate::server::shared::types::api::{ApiError, ApiErrorResponse, ApiResponse, ApiResult};

impl CrudHandlers for CustomTopologyView {
    type Service = CustomTopologyViewService;
    type FilterQuery = NetworkFilterQuery;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.custom_topology_view_service
    }
}

mod generated {
    use super::*;
    crate::crud_get_all_handler!(CustomTopologyView);
    crate::crud_get_by_id_handler!(CustomTopologyView);
    crate::crud_create_handler!(CustomTopologyView);
    crate::crud_update_handler!(CustomTopologyView);
    crate::crud_delete_handler!(CustomTopologyView);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(generated::get_all, generated::create))
        .routes(routes!(
            generated::get_by_id,
            generated::update,
            generated::delete
        ))
        .routes(routes!(save_custom_topology_view_layout))
}

/// Batch-upsert positions/styles for many nodes and edges on one view in a
/// single request — the endpoint the frontend's debounced auto-save (on
/// drag-stop, resize-stop, edit-blur) calls, so moving several nodes at once
/// doesn't fire one HTTP round trip per node. Each item with a nil `id` is
/// created; every other item is updated in place. Does not delete anything
/// omitted from the payload — use the per-node/per-edge DELETE endpoints for
/// removal.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SaveLayoutRequest {
    #[serde(default)]
    pub nodes: Vec<CustomViewNode>,
    #[serde(default)]
    pub edges: Vec<CustomViewEdge>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SaveLayoutResponse {
    pub nodes: Vec<CustomViewNode>,
    pub edges: Vec<CustomViewEdge>,
}

#[utoipa::path(
    put,
    path = "/{id}/layout",
    tag = CustomTopologyView::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Custom topology view ID")),
    request_body = SaveLayoutRequest,
    responses(
        (status = 200, description = "Layout saved", body = ApiResponse<SaveLayoutResponse>),
        (status = 404, description = "View not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn save_custom_topology_view_layout(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
    Json(payload): Json<SaveLayoutRequest>,
) -> ApiResult<Json<ApiResponse<SaveLayoutResponse>>> {
    let view = state
        .services
        .custom_topology_view_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<CustomTopologyView>(id))?;
    if !auth.network_ids().contains(&view.base.network_id) {
        return Err(ApiError::entity_not_found::<CustomTopologyView>(id));
    }
    let authenticated = auth.into_entity();

    let mut saved_nodes = Vec::with_capacity(payload.nodes.len());
    for mut node in payload.nodes {
        node.base.view_id = id;
        node.base.network_id = view.base.network_id;
        let saved = if node.id() == Uuid::nil() {
            state
                .services
                .custom_view_node_service
                .create(node, authenticated.clone())
                .await
        } else {
            state
                .services
                .custom_view_node_service
                .update(&mut node, authenticated.clone())
                .await
        }
        .map_err(|e| ApiError::internal_error(&format!("Failed to save node: {e}")))?;
        saved_nodes.push(saved);
    }

    let mut saved_edges = Vec::with_capacity(payload.edges.len());
    for mut edge in payload.edges {
        edge.base.view_id = id;
        edge.base.network_id = view.base.network_id;
        let saved = if edge.id() == Uuid::nil() {
            state
                .services
                .custom_view_edge_service
                .create(edge, authenticated.clone())
                .await
        } else {
            state
                .services
                .custom_view_edge_service
                .update(&mut edge, authenticated.clone())
                .await
        }
        .map_err(|e| ApiError::internal_error(&format!("Failed to save edge: {e}")))?;
        saved_edges.push(saved);
    }

    Ok(Json(ApiResponse::success(SaveLayoutResponse {
        nodes: saved_nodes,
        edges: saved_edges,
    })))
}

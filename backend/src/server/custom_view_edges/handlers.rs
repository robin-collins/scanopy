use axum::Json;
use axum::extract::State;
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::server::auth::middleware::permissions::{Authorized, Member};
use crate::server::config::AppState;
use crate::server::custom_topology_views::r#impl::base::CustomTopologyView;
use crate::server::custom_view_edges::{
    r#impl::base::CustomViewEdge, service::CustomViewEdgeService,
};
use crate::server::shared::handlers::query::CustomViewChildQuery;
use crate::server::shared::handlers::traits::{CrudHandlers, create_handler};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::traits::Entity;
use crate::server::shared::types::api::{ApiError, ApiErrorResponse, ApiResponse, ApiResult};

impl CrudHandlers for CustomViewEdge {
    type Service = CustomViewEdgeService;
    type FilterQuery = CustomViewChildQuery;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.custom_view_edge_service
    }
}

mod generated {
    use super::*;
    crate::crud_get_all_handler!(CustomViewEdge);
    crate::crud_get_by_id_handler!(CustomViewEdge);
    crate::crud_update_handler!(CustomViewEdge);
    crate::crud_delete_handler!(CustomViewEdge);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(generated::get_all, create_custom_view_edge))
        .routes(routes!(
            generated::get_by_id,
            generated::update,
            generated::delete
        ))
}

/// Create an edge between two nodes on a custom view. `network_id` is
/// derived from the parent view server-side, and both endpoints are checked
/// to actually belong to that view — an edge can't span two different views.
#[utoipa::path(
    post,
    path = "",
    tag = CustomViewEdge::ENTITY_NAME_PLURAL,
    request_body = CustomViewEdge,
    responses(
        (status = 200, description = "Edge created successfully", body = ApiResponse<CustomViewEdge>),
        (status = 400, description = "Validation error, or endpoints not on the same view", body = ApiErrorResponse),
        (status = 404, description = "Custom topology view not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn create_custom_view_edge(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Json(mut edge): Json<CustomViewEdge>,
) -> ApiResult<Json<ApiResponse<CustomViewEdge>>> {
    let view = state
        .services
        .custom_topology_view_service
        .get_by_id(&edge.base.view_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<CustomTopologyView>(edge.base.view_id))?;
    if !auth.network_ids().contains(&view.base.network_id) {
        return Err(ApiError::entity_not_found::<CustomTopologyView>(
            edge.base.view_id,
        ));
    }
    edge.base.network_id = view.base.network_id;

    for node_id in [edge.base.source_node_id, edge.base.target_node_id] {
        let node = state
            .services
            .custom_view_node_service
            .get_by_id(&node_id)
            .await?
            .ok_or_else(|| ApiError::bad_request("Edge endpoint node not found"))?;
        if node.base.view_id != edge.base.view_id {
            return Err(ApiError::bad_request(
                "Edge endpoints must both belong to the same view",
            ));
        }
    }

    create_handler::<CustomViewEdge>(State(state), auth, Json(edge)).await
}

use axum::Json;
use axum::body::Body;
use axum::extract::{Multipart, Path, State};
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::server::auth::middleware::permissions::{Authorized, Member, Viewer};
use crate::server::config::AppState;
use crate::server::custom_topology_views::r#impl::base::CustomTopologyView;
use crate::server::custom_view_nodes::{
    r#impl::base::CustomViewNode, service::CustomViewNodeService,
};
use crate::server::shared::handlers::query::CustomViewChildQuery;
use crate::server::shared::handlers::traits::{CrudHandlers, create_handler};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::traits::Entity;
use crate::server::shared::types::api::{ApiError, ApiErrorResponse, ApiResponse, ApiResult};

/// Bounds how much of a multipart body we buffer in memory per upload — same
/// limit as host image / library object uploads.
const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;

impl CrudHandlers for CustomViewNode {
    type Service = CustomViewNodeService;
    type FilterQuery = CustomViewChildQuery;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.custom_view_node_service
    }
}

mod generated {
    use super::*;
    crate::crud_get_all_handler!(CustomViewNode);
    crate::crud_get_by_id_handler!(CustomViewNode);
    crate::crud_update_handler!(CustomViewNode);
    crate::crud_delete_handler!(CustomViewNode);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(generated::get_all, create_custom_view_node))
        .routes(routes!(
            generated::get_by_id,
            generated::update,
            generated::delete
        ))
        .routes(routes!(upload_custom_view_node_image))
        .routes(routes!(get_custom_view_node_image_content))
}

/// Create a node on a custom view. `network_id` is always derived from the
/// parent view server-side (never trusted from the client) so it can't drift
/// from the denormalized-scoping invariant every child-query filter relies on.
#[utoipa::path(
    post,
    path = "",
    tag = CustomViewNode::ENTITY_NAME_PLURAL,
    request_body = CustomViewNode,
    responses(
        (status = 200, description = "Node created successfully", body = ApiResponse<CustomViewNode>),
        (status = 400, description = "Validation error", body = ApiErrorResponse),
        (status = 404, description = "Custom topology view not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn create_custom_view_node(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Json(mut node): Json<CustomViewNode>,
) -> ApiResult<Json<ApiResponse<CustomViewNode>>> {
    let view = state
        .services
        .custom_topology_view_service
        .get_by_id(&node.base.view_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<CustomTopologyView>(node.base.view_id))?;
    if !auth.network_ids().contains(&view.base.network_id) {
        return Err(ApiError::entity_not_found::<CustomTopologyView>(
            node.base.view_id,
        ));
    }
    node.base.network_id = view.base.network_id;

    create_handler::<CustomViewNode>(State(state), auth, Json(node)).await
}

/// Upload (or replace) a per-node custom image override.
#[utoipa::path(
    put,
    path = "/{id}/image",
    tag = CustomViewNode::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Custom view node ID")),
    responses(
        (status = 200, description = "Image uploaded successfully", body = ApiResponse<CustomViewNode>),
        (status = 400, description = "Missing file, unsupported type, or file too large", body = ApiErrorResponse),
        (status = 404, description = "Node not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn upload_custom_view_node_image(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> ApiResult<Json<ApiResponse<CustomViewNode>>> {
    let mut node = state
        .services
        .custom_view_node_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<CustomViewNode>(id))?;
    if !auth.network_ids().contains(&node.base.network_id) {
        return Err(ApiError::entity_not_found::<CustomViewNode>(id));
    }

    let mut bytes: Option<Vec<u8>> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(&format!("Invalid multipart body: {e}")))?
    {
        if field.name() == Some("file") {
            let data = field
                .bytes()
                .await
                .map_err(|e| ApiError::bad_request(&format!("Failed to read file: {e}")))?;
            if data.len() > MAX_IMAGE_BYTES {
                return Err(ApiError::bad_request(&format!(
                    "Image exceeds the {MAX_IMAGE_BYTES}-byte limit"
                )));
            }
            bytes = Some(data.to_vec());
        }
    }
    let bytes = bytes.ok_or_else(|| ApiError::bad_request("Missing file field"))?;

    let sniffed = infer::get(&bytes)
        .filter(|t| t.matcher_type() == infer::MatcherType::Image)
        .ok_or_else(|| ApiError::bad_request("File is not a recognized image format"))?;

    let storage_path = format!("custom-view-nodes/{id}.{}", sniffed.extension());
    let absolute_path = state
        .services
        .custom_view_node_service
        .absolute_path(&storage_path);
    if let Some(parent) = absolute_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            ApiError::internal_error(&format!("Failed to create image directory: {e}"))
        })?;
    }
    tokio::fs::write(&absolute_path, &bytes)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to write image: {e}")))?;

    node.base.content_type = Some(sniffed.mime_type().to_string());
    node.base.size_bytes = Some(bytes.len() as i64);
    node.base.storage_path = Some(storage_path);

    let updated = state
        .services
        .custom_view_node_service
        .update(&mut node, auth.into_entity())
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to save image record: {e}")))?;

    Ok(Json(ApiResponse::success(updated)))
}

/// Stream a custom view node's uploaded image bytes, if any.
#[utoipa::path(
    get,
    path = "/{id}/content",
    tag = CustomViewNode::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Custom view node ID")),
    responses(
        (status = 200, description = "Image bytes", content_type = "application/octet-stream", body = [u8]),
        (status = 404, description = "Node or image not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_custom_view_node_image_content(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    let node = state
        .services
        .custom_view_node_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<CustomViewNode>(id))?;
    if !auth.network_ids().contains(&node.base.network_id) {
        return Err(ApiError::entity_not_found::<CustomViewNode>(id));
    }
    let storage_path = node
        .base
        .storage_path
        .as_ref()
        .ok_or_else(|| ApiError::entity_not_found::<CustomViewNode>(id))?;

    let bytes = tokio::fs::read(
        state
            .services
            .custom_view_node_service
            .absolute_path(storage_path),
    )
    .await
    .map_err(|e| ApiError::internal_error(&format!("Failed to read image: {e}")))?;

    let mut response = Body::from(bytes).into_response();
    if let Some(content_type) = &node.base.content_type
        && let Ok(value) = HeaderValue::from_str(content_type)
    {
        response.headers_mut().insert(header::CONTENT_TYPE, value);
    }
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=86400"),
    );
    Ok(response)
}

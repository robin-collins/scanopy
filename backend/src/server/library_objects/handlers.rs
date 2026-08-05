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
use crate::server::library_objects::{r#impl::base::LibraryObject, service::LibraryObjectService};
use crate::server::shared::handlers::query::NoFilterQuery;
use crate::server::shared::handlers::traits::{CrudHandlers, delete_handler, update_handler};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::Entity;
use crate::server::shared::types::api::{
    ApiError, ApiErrorResponse, ApiResponse, ApiResult, EmptyApiResponse, PaginatedApiResponse,
};

/// Bounds how much of a multipart body we buffer in memory per upload — same
/// limit as host image uploads.
const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;

impl CrudHandlers for LibraryObject {
    type Service = LibraryObjectService;
    type FilterQuery = NoFilterQuery;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.library_object_service
    }
}

mod generated {
    use super::*;
    crate::crud_get_by_id_handler!(LibraryObject);
    crate::crud_create_handler!(LibraryObject);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(list_library_objects, generated::create))
        .routes(routes!(
            generated::get_by_id,
            update_library_object,
            delete_library_object
        ))
        .routes(routes!(upload_library_object_image))
        .routes(routes!(get_library_object_image_content))
}

/// List every library object visible to the caller's organization: the
/// seeded built-in catalog (`organization_id IS NULL`) plus that org's own
/// additions. Bypasses the generic `get_all_handler` — its automatic
/// org-scoped base filter (`organization_id = $1`) can't express "or NULL",
/// which this entity's shared-catalog model needs.
#[utoipa::path(
    get,
    path = "",
    tag = LibraryObject::ENTITY_NAME_PLURAL,
    responses(
        (status = 200, description = "List of library objects", body = inline(PaginatedApiResponse<LibraryObject>)),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn list_library_objects(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
) -> ApiResult<Json<PaginatedApiResponse<LibraryObject>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    let filter = StorableFilter::<LibraryObject>::new_from_org_id_or_null(&organization_id);
    let result = state
        .services
        .library_object_service
        .get_paginated(filter)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    Ok(Json(PaginatedApiResponse::success(
        result.items,
        result.total_count,
        result.total_count as u32,
        0,
    )))
}

/// Reject edits to a built-in (`organization_id IS NULL`) row before
/// delegating to the generic update handler.
#[utoipa::path(
    put,
    path = "/{id}",
    tag = LibraryObject::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Library object ID")),
    request_body = LibraryObject,
    responses(
        (status = 200, description = "Library object updated", body = ApiResponse<LibraryObject>),
        (status = 400, description = "Built-in library objects cannot be modified", body = ApiErrorResponse),
        (status = 404, description = "Library object not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn update_library_object(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    path: Path<Uuid>,
    body: Json<LibraryObject>,
) -> ApiResult<Json<ApiResponse<LibraryObject>>> {
    reject_builtin(&state, &path.0).await?;
    update_handler::<LibraryObject>(State(state), auth, path, body).await
}

/// Reject deletion of a built-in row, then delegate to the generic delete
/// handler (which also cleans up any uploaded image row — the on-disk file
/// removal below runs first, best-effort).
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = LibraryObject::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Library object ID")),
    responses(
        (status = 200, description = "Library object deleted", body = EmptyApiResponse),
        (status = 400, description = "Built-in library objects cannot be deleted", body = ApiErrorResponse),
        (status = 404, description = "Library object not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn delete_library_object(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    path: Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    reject_builtin(&state, &path.0).await?;
    if let Some(object) = state
        .services
        .library_object_service
        .get_by_id(&path.0)
        .await?
        && let Some(storage_path) = &object.base.storage_path
    {
        let absolute_path = state
            .services
            .library_object_service
            .absolute_path(storage_path);
        let _ = tokio::fs::remove_file(absolute_path).await;
    }
    delete_handler::<LibraryObject>(State(state), auth, path).await
}

async fn reject_builtin(state: &Arc<AppState>, id: &Uuid) -> ApiResult<()> {
    if let Some(existing) = state.services.library_object_service.get_by_id(id).await?
        && existing.base.organization_id.is_none()
    {
        return Err(ApiError::bad_request(
            "Built-in library objects cannot be modified or deleted",
        ));
    }
    Ok(())
}

/// Upload (or replace) the image for an existing library object. Multipart
/// field: `file`. Rejects uploads onto a built-in row, same as update/delete.
#[utoipa::path(
    put,
    path = "/{id}/image",
    tag = LibraryObject::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Library object ID")),
    responses(
        (status = 200, description = "Image uploaded successfully", body = ApiResponse<LibraryObject>),
        (status = 400, description = "Missing file, unsupported type, file too large, or built-in object", body = ApiErrorResponse),
        (status = 404, description = "Library object not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn upload_library_object_image(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> ApiResult<Json<ApiResponse<LibraryObject>>> {
    let mut object = state
        .services
        .library_object_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<LibraryObject>(id))?;
    if object.base.organization_id != auth.organization_id() {
        return Err(ApiError::entity_not_found::<LibraryObject>(id));
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

    let storage_path = format!("library-objects/{id}.{}", sniffed.extension());
    let absolute_path = state
        .services
        .library_object_service
        .absolute_path(&storage_path);
    if let Some(parent) = absolute_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            ApiError::internal_error(&format!("Failed to create image directory: {e}"))
        })?;
    }
    tokio::fs::write(&absolute_path, &bytes)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to write image: {e}")))?;

    object.base.content_type = Some(sniffed.mime_type().to_string());
    object.base.size_bytes = Some(bytes.len() as i64);
    object.base.storage_path = Some(storage_path);

    let updated = state
        .services
        .library_object_service
        .update(&mut object, auth.into_entity())
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to save image record: {e}")))?;

    Ok(Json(ApiResponse::success(updated)))
}

/// Stream a library object's uploaded image bytes, if any.
#[utoipa::path(
    get,
    path = "/{id}/content",
    tag = LibraryObject::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Library object ID")),
    responses(
        (status = 200, description = "Image bytes", content_type = "application/octet-stream", body = [u8]),
        (status = 404, description = "Library object or image not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_library_object_image_content(
    State(state): State<Arc<AppState>>,
    _auth: Authorized<Viewer>,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    let object = state
        .services
        .library_object_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<LibraryObject>(id))?;
    let storage_path = object
        .base
        .storage_path
        .as_ref()
        .ok_or_else(|| ApiError::entity_not_found::<LibraryObject>(id))?;

    let bytes = tokio::fs::read(
        state
            .services
            .library_object_service
            .absolute_path(storage_path),
    )
    .await
    .map_err(|e| ApiError::internal_error(&format!("Failed to read image: {e}")))?;

    let mut response = Body::from(bytes).into_response();
    if let Some(content_type) = &object.base.content_type
        && let Ok(value) = HeaderValue::from_str(content_type)
    {
        response.headers_mut().insert(header::CONTENT_TYPE, value);
    }
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=86400"),
    );
    Ok(response)
}

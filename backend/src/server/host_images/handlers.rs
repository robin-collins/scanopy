use axum::Json;
use axum::body::Body;
use axum::extract::{Multipart, Path, State};
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::server::auth::middleware::permissions::{Authorized, Member};
use crate::server::config::AppState;
use crate::server::host_images::{r#impl::base::HostImage, service::HostImageService};
use crate::server::hosts::r#impl::base::Host;
use crate::server::shared::handlers::query::HostChildQuery;
use crate::server::shared::handlers::traits::{CrudHandlers, delete_handler};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::traits::{Entity, Storable};
use crate::server::shared::types::api::{
    ApiError, ApiErrorResponse, ApiResponse, ApiResult, EmptyApiResponse,
};

/// Bounds how much of a multipart body we buffer in memory per upload.
/// Generous enough for real photos, small enough that a handful of
/// concurrent uploads can't exhaust server memory.
const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;

impl CrudHandlers for HostImage {
    type Service = HostImageService;
    type FilterQuery = HostChildQuery;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.host_image_service
    }
}

mod generated {
    use super::*;
    crate::crud_get_all_handler!(HostImage);
    crate::crud_get_by_id_handler!(HostImage);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(generated::get_all, upload_host_image))
        .routes(routes!(generated::get_by_id, delete_host_image))
        .routes(routes!(get_host_image_content))
}

/// Upload a new image to a host's gallery. Multipart fields: `host_id`
/// (text) and `file` (the image). The declared content-type on the `file`
/// field is advisory only — the stored content-type comes from sniffing the
/// actual bytes (`infer`), so a mislabeled or spoofed upload can't get
/// served back with a misleading `Content-Type` header later.
#[utoipa::path(
    post,
    path = "",
    tag = HostImage::ENTITY_NAME_PLURAL,
    responses(
        (status = 200, description = "Image uploaded successfully", body = ApiResponse<HostImage>),
        (status = 400, description = "Missing host_id/file, unsupported type, or file too large", body = ApiErrorResponse),
        (status = 404, description = "Host not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn upload_host_image(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    mut multipart: Multipart,
) -> ApiResult<Json<ApiResponse<HostImage>>> {
    let mut host_id: Option<Uuid> = None;
    let mut filename: Option<String> = None;
    let mut bytes: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(&format!("Invalid multipart body: {e}")))?
    {
        match field.name() {
            Some("host_id") => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| ApiError::bad_request(&format!("Invalid host_id field: {e}")))?;
                host_id = Some(
                    Uuid::parse_str(text.trim())
                        .map_err(|_| ApiError::bad_request("host_id is not a valid UUID"))?,
                );
            }
            Some("file") => {
                filename = field.file_name().map(|s| s.to_string());
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
            _ => {} // Ignore unrecognized fields rather than rejecting the whole upload.
        }
    }

    let host_id = host_id.ok_or_else(|| ApiError::bad_request("Missing host_id field"))?;
    let bytes = bytes.ok_or_else(|| ApiError::bad_request("Missing file field"))?;
    let filename = filename.unwrap_or_else(|| "upload".to_string());

    let host = state
        .services
        .host_service
        .get_by_id(&host_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Host>(host_id))?;
    if !auth.network_ids().contains(&host.base.network_id) {
        return Err(ApiError::entity_not_found::<Host>(host_id));
    }

    let sniffed = infer::get(&bytes)
        .filter(|t| t.matcher_type() == infer::MatcherType::Image)
        .ok_or_else(|| ApiError::bad_request("File is not a recognized image format"))?;

    let image_id = Uuid::new_v4();
    let storage_path = format!("host-images/{host_id}/{image_id}.{}", sniffed.extension());
    let absolute_path = state.services.host_image_service.data_dir().join(&storage_path);
    if let Some(parent) = absolute_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| ApiError::internal_error(&format!("Failed to create image directory: {e}")))?;
    }
    tokio::fs::write(&absolute_path, &bytes)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to write image: {e}")))?;

    let mut image = HostImage::new(crate::server::host_images::r#impl::base::HostImageBase {
        host_id,
        network_id: host.base.network_id,
        filename,
        content_type: sniffed.mime_type().to_string(),
        size_bytes: bytes.len() as i64,
        storage_path,
    });
    image.id = image_id;

    let created = state
        .services
        .host_image_service
        .create(image, auth.into_entity())
        .await
        .map_err(|e| {
            // Best-effort: don't leave an orphaned file if the DB insert failed.
            let path = absolute_path.clone();
            tokio::spawn(async move {
                let _ = tokio::fs::remove_file(path).await;
            });
            ApiError::internal_error(&format!("Failed to save image record: {e}"))
        })?;

    Ok(Json(ApiResponse::success(created)))
}

/// Delete a host image: removes the on-disk file (best-effort — a missing
/// file shouldn't block removing the now-orphaned DB row) then delegates to
/// the generic delete handler for the permission-checked DB delete.
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = HostImage::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Host image ID")),
    responses(
        (status = 200, description = "Image deleted successfully", body = EmptyApiResponse),
        (status = 404, description = "Image not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn delete_host_image(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    path: Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    if let Some(image) = state.services.host_image_service.get_by_id(&path.0).await? {
        let absolute_path = state.services.host_image_service.absolute_path(&image);
        let _ = tokio::fs::remove_file(absolute_path).await;
    }
    delete_handler::<HostImage>(State(state), auth, path).await
}

/// Stream a host image's raw bytes with its sniffed content-type. Not part
/// of the generic CRUD surface (it returns a binary body, not `ApiResponse<T>`).
#[utoipa::path(
    get,
    path = "/{id}/content",
    tag = HostImage::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Host image ID")),
    responses(
        (status = 200, description = "Image bytes", content_type = "application/octet-stream"),
        (status = 404, description = "Image not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_host_image_content(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    let image = state
        .services
        .host_image_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<HostImage>(id))?;
    if !auth.network_ids().contains(&image.base.network_id) {
        return Err(ApiError::entity_not_found::<HostImage>(id));
    }

    let bytes = tokio::fs::read(state.services.host_image_service.absolute_path(&image))
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to read image: {e}")))?;

    let mut response = Body::from(bytes).into_response();
    if let Ok(value) = HeaderValue::from_str(&image.base.content_type) {
        response.headers_mut().insert(header::CONTENT_TYPE, value);
    }
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=86400"),
    );
    Ok(response)
}

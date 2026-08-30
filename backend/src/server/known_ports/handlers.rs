use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::server::{
    auth::middleware::permissions::{Authorized, Member, Viewer},
    config::AppState,
    known_ports::{
        service::KnownPortServiceError,
        types::{KnownPort, KnownPortInput},
    },
    openapi::tags as api_tags,
    shared::types::api::{ApiError, ApiErrorResponse, ApiResponse, ApiResult, EmptyApiResponse},
};

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(list_known_ports, create_known_port))
        .routes(routes!(update_known_port, delete_known_port))
}

#[utoipa::path(
    get,
    path = "",
    tag = api_tags::KNOWN_PORTS,
    responses((status = 200, description = "Visible built-in and custom known ports", body = ApiResponse<Vec<KnownPort>>)),
    security(("user_api_key" = []), ("session" = []))
)]
async fn list_known_ports(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
) -> ApiResult<Json<ApiResponse<Vec<KnownPort>>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let ports = state
        .services
        .known_port_service
        .list(organization_id)
        .await
        .map_err(map_service_error)?;
    Ok(Json(ApiResponse::success(ports)))
}

#[utoipa::path(
    post,
    path = "",
    tag = api_tags::KNOWN_PORTS,
    request_body = KnownPortInput,
    responses(
        (status = 200, description = "Custom known port created", body = ApiResponse<KnownPort>),
        (status = 400, description = "Invalid or conflicting definition", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn create_known_port(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Json(input): Json<KnownPortInput>,
) -> ApiResult<Json<ApiResponse<KnownPort>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let port = state
        .services
        .known_port_service
        .create(organization_id, input)
        .await
        .map_err(map_service_error)?;
    Ok(Json(ApiResponse::success(port)))
}

#[utoipa::path(
    put,
    path = "/custom/{id}",
    tag = api_tags::KNOWN_PORTS,
    params(("id" = Uuid, Path, description = "Known port ID")),
    request_body = KnownPortInput,
    responses(
        (status = 200, description = "Custom known port updated", body = ApiResponse<KnownPort>),
        (status = 400, description = "Invalid or conflicting definition", body = ApiErrorResponse),
        (status = 404, description = "Known port not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn update_known_port(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
    Json(input): Json<KnownPortInput>,
) -> ApiResult<Json<ApiResponse<KnownPort>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let port = state
        .services
        .known_port_service
        .update(organization_id, id, input)
        .await
        .map_err(map_service_error)?;
    Ok(Json(ApiResponse::success(port)))
}

#[utoipa::path(
    delete,
    path = "/custom/{id}",
    tag = api_tags::KNOWN_PORTS,
    params(("id" = Uuid, Path, description = "Known port ID")),
    responses(
        (status = 200, description = "Custom known port deleted", body = EmptyApiResponse),
        (status = 404, description = "Known port not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn delete_known_port(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<EmptyApiResponse>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    state
        .services
        .known_port_service
        .delete(organization_id, id)
        .await
        .map_err(map_service_error)?;
    Ok(Json(ApiResponse::success(())))
}

fn map_service_error(error: KnownPortServiceError) -> ApiError {
    match error {
        KnownPortServiceError::NotFound => ApiError::not_found("Known port not found".to_string()),
        KnownPortServiceError::Conflict | KnownPortServiceError::Validation(_) => {
            ApiError::bad_request(&error.to_string())
        }
        KnownPortServiceError::Database(database_error) => {
            ApiError::internal_error(&database_error.to_string())
        }
    }
}

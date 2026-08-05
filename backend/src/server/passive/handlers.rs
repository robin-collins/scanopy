use std::sync::Arc;

use axum::{
    Json,
    extract::{DefaultBodyLimit, State},
};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::server::{
    auth::middleware::permissions::{Authorized, IsDaemon, Viewer},
    config::AppState,
    shared::{
        extractors::Query,
        types::api::{ApiError, ApiErrorResponse, ApiResponse, ApiResult, PaginatedApiResponse},
    },
};

use super::{
    storage,
    types::{
        MAX_REQUEST_BODY_BYTES, PassiveIngestRequest, PassiveIngestResponse, PassiveListQuery,
        PassiveObservation,
    },
};

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(list_observations, ingest_observations))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BODY_BYTES))
}

#[utoipa::path(
    get, path = "/observations", tag = "Passive observations",
    params(PassiveListQuery),
    responses((status = 200, body = PaginatedApiResponse<PassiveObservation>)),
    security(("user_api_key" = []), ("session" = []))
)]
async fn list_observations(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Query(query): Query<PassiveListQuery>,
) -> ApiResult<Json<PaginatedApiResponse<PassiveObservation>>> {
    let (items, total) = storage::list(&state.pool, &auth.network_ids(), &query)
        .await
        .map_err(storage_error)?;
    let (limit, offset) = query.pagination();
    Ok(Json(PaginatedApiResponse::success(
        items, total, limit, offset,
    )))
}

#[utoipa::path(
    post, path = "/observations", tag = "Passive observations",
    request_body = PassiveIngestRequest,
    responses(
        (status = 200, body = ApiResponse<PassiveIngestResponse>),
        (status = 400, body = ApiErrorResponse),
        (status = 403, body = ApiErrorResponse)
    ),
    security(("daemon_api_key" = []))
)]
async fn ingest_observations(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsDaemon>,
    Json(request): Json<PassiveIngestRequest>,
) -> ApiResult<Json<ApiResponse<PassiveIngestResponse>>> {
    request
        .validate()
        .map_err(|message| ApiError::bad_request(&message))?;
    if !auth.network_ids().contains(&request.network_id) {
        return Err(ApiError::forbidden(
            "Daemon cannot persist passive observations for an unassigned network",
        ));
    }
    let daemon_id = auth
        .daemon_id()
        .ok_or_else(|| ApiError::forbidden("Daemon identity is required"))?;
    let response = storage::ingest(&state.pool, request.network_id, daemon_id, &request)
        .await
        .map_err(storage_error)?;
    Ok(Json(ApiResponse::success(response)))
}

fn storage_error(error: anyhow::Error) -> ApiError {
    tracing::error!(error = %error, "Passive observation persistence failed");
    ApiError::internal_error("Passive observation persistence failed")
}

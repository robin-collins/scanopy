use axum::Json;
use axum::extract::{Path, State};
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::server::auth::middleware::permissions::{Authorized, Member, Viewer};
use crate::server::categories::{r#impl::base::Category, service::CategoryService};
use crate::server::config::AppState;
use crate::server::shared::handlers::query::NoFilterQuery;
use crate::server::shared::handlers::traits::{CrudHandlers, delete_handler, update_handler};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::Entity;
use crate::server::shared::types::api::{
    ApiError, ApiErrorResponse, ApiResponse, ApiResult, EmptyApiResponse, PaginatedApiResponse,
};

impl CrudHandlers for Category {
    type Service = CategoryService;
    type FilterQuery = NoFilterQuery;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.category_service
    }
}

mod generated {
    use super::*;
    crate::crud_get_by_id_handler!(Category);
    crate::crud_create_handler!(Category);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(list_categories, generated::create))
        .routes(routes!(
            generated::get_by_id,
            update_category,
            delete_category
        ))
}

/// List every category visible to the caller's organization: the seeded
/// built-in catalog (`organization_id IS NULL`) plus that org's own
/// additions. Bypasses the generic `get_all_handler` — its automatic
/// org-scoped base filter (`organization_id = $1`) can't express "or NULL",
/// which this entity's shared-catalog model needs.
#[utoipa::path(
    get,
    path = "",
    tag = Category::ENTITY_NAME_PLURAL,
    responses(
        (status = 200, description = "List of categories", body = inline(PaginatedApiResponse<Category>)),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn list_categories(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
) -> ApiResult<Json<PaginatedApiResponse<Category>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    let filter = StorableFilter::<Category>::new_from_org_id_or_null(&organization_id);
    let result = state
        .services
        .category_service
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

/// Reject edits to a built-in (`organization_id IS NULL`) category before
/// delegating to the generic update handler.
#[utoipa::path(
    put,
    path = "/{id}",
    tag = Category::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Category ID")),
    request_body = Category,
    responses(
        (status = 200, description = "Category updated", body = ApiResponse<Category>),
        (status = 400, description = "Built-in categories cannot be modified", body = ApiErrorResponse),
        (status = 404, description = "Category not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn update_category(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    path: Path<Uuid>,
    body: Json<Category>,
) -> ApiResult<Json<ApiResponse<Category>>> {
    reject_builtin(&state, &path.0).await?;
    update_handler::<Category>(State(state), auth, path, body).await
}

/// Reject deletion of a built-in category, then delegate to the generic
/// delete handler. Hosts referencing the deleted category fall back to
/// uncategorized via `ON DELETE SET NULL`.
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = Category::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Category ID")),
    responses(
        (status = 200, description = "Category deleted", body = EmptyApiResponse),
        (status = 400, description = "Built-in categories cannot be deleted", body = ApiErrorResponse),
        (status = 404, description = "Category not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn delete_category(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    path: Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    reject_builtin(&state, &path.0).await?;
    delete_handler::<Category>(State(state), auth, path).await
}

async fn reject_builtin(state: &Arc<AppState>, id: &Uuid) -> ApiResult<()> {
    if let Some(existing) = state.services.category_service.get_by_id(id).await?
        && existing.base.organization_id.is_none()
    {
        return Err(ApiError::bad_request(
            "Built-in categories cannot be modified or deleted",
        ));
    }
    Ok(())
}

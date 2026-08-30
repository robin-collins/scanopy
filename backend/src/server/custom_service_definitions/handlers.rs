use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use strum::IntoEnumIterator;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::server::auth::middleware::permissions::{Authorized, Member, Viewer};
use crate::server::config::AppState;
use crate::server::custom_service_definitions::{
    r#impl::base::CustomServiceDefinition, service::CustomServiceDefinitionService,
};
use crate::server::services::definitions::ServiceDefinitionRegistry;
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::shared::handlers::query::NoFilterQuery;
use crate::server::shared::handlers::traits::{CrudHandlers, delete_handler, update_handler};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::Entity;
use crate::server::shared::types::api::{
    ApiError, ApiErrorResponse, ApiResponse, ApiResult, EmptyApiResponse, PaginatedApiResponse,
};
use crate::server::shared::types::metadata::{EntityMetadataProvider, HasId};

/// Which namespace a catalogue entry lives in. Built-in definitions are
/// compile-time Rust types (read-only, no rows); custom definitions are DB
/// rows with full CRUD.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ServiceCatalogueEntryKind {
    BuiltIn,
    Custom,
}

/// A single entry in the merged service catalogue — built-in definitions from
/// the compile-time `ServiceDefinitionRegistry` plus user-created custom
/// definitions from `custom_service_definitions`. The backend owns this merge
/// so every consumer (the Known Services page, service pickers, per-host port
/// overrides in #10) shares one resolution rule.
///
/// The `id` is a unique reference across both namespaces: custom names are
/// validated to never collide (case-insensitively) with a built-in id, so a
/// bare string is never ambiguous. `custom_id` carries the DB row id so CRUD
/// can target a custom entry.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ServiceCatalogueEntry {
    pub kind: ServiceCatalogueEntryKind,
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub logo_url: String,
    pub logo_needs_white_background: bool,
    pub is_generic: bool,
    pub custom_id: Option<Uuid>,
}

impl CrudHandlers for CustomServiceDefinition {
    type Service = CustomServiceDefinitionService;
    type FilterQuery = NoFilterQuery;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.custom_service_definition_service
    }
}

mod generated {
    use super::*;
    crate::crud_get_by_id_handler!(CustomServiceDefinition);
    crate::crud_create_handler!(CustomServiceDefinition);
}

/// CRUD router for custom service definitions, mounted at
/// `/api/v1/custom-service-definitions`.
pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(list_custom_service_definitions, generated::create))
        .routes(routes!(
            generated::get_by_id,
            update_custom_service_definition,
            delete_custom_service_definition
        ))
}

/// Read-only merged service catalogue, mounted at `/api/v1/service-catalogue`.
pub fn create_catalogue_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new().routes(routes!(get_service_catalogue))
}

/// List the caller's organization's custom service definitions.
#[utoipa::path(
    get,
    path = "",
    tag = CustomServiceDefinition::ENTITY_NAME_PLURAL,
    responses(
        (status = 200, description = "List of custom service definitions", body = inline(PaginatedApiResponse<CustomServiceDefinition>)),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn list_custom_service_definitions(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
) -> ApiResult<Json<PaginatedApiResponse<CustomServiceDefinition>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    // Tenant-scoped: a custom service from another org must be invisible, not
    // merely unlisted.
    let filter = StorableFilter::<CustomServiceDefinition>::new_from_org_id(&organization_id);
    let result = state
        .services
        .custom_service_definition_service
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

/// Update a custom service definition. Built-in definitions are compile-time
/// and have no rows, so nothing here can touch them; validation still runs so
/// a custom row cannot be renamed onto a built-in id or given garbage data.
/// The generic update handler enforces tenant scoping (the existing row's
/// organization must match the caller's).
#[utoipa::path(
    put,
    path = "/{id}",
    tag = CustomServiceDefinition::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Custom service definition ID")),
    request_body = CustomServiceDefinition,
    responses(
        (status = 200, description = "Custom service definition updated", body = ApiResponse<CustomServiceDefinition>),
        (status = 400, description = "Validation error: invalid category, built-in name collision, or duplicate name", body = ApiErrorResponse),
        (status = 404, description = "Custom service definition not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn update_custom_service_definition(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    path: axum::extract::Path<Uuid>,
    mut body: Json<CustomServiceDefinition>,
) -> ApiResult<Json<ApiResponse<CustomServiceDefinition>>> {
    CustomServiceDefinitionService::validate_custom_definition(&mut body.0)?;
    update_handler::<CustomServiceDefinition>(State(state), auth, path, body).await
}

/// Delete a custom service definition. Built-in definitions have no rows, so
/// there is nothing to protect beyond the custom table itself.
///
/// INVARIANT 5: a custom entry that is referenced by a host override must be
/// BLOCKED from deletion — not cascaded, not nulled — so the user's
/// classification is never silently lost. The referencing hosts are resolved
/// within the caller's organization only (a reference from another org must
/// not leak or block).
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = CustomServiceDefinition::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Custom service definition ID")),
    responses(
        (status = 200, description = "Custom service definition deleted", body = EmptyApiResponse),
        (status = 404, description = "Custom service definition not found", body = ApiErrorResponse),
        (status = 409, description = "Custom service definition is referenced by host overrides and cannot be deleted", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn delete_custom_service_definition(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    path: axum::extract::Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    // Resolve the row first so we have its catalogue id (name) to check
    // references against. Tenant scoping is enforced by the org filter here:
    // a cross-org id resolves to nothing.
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    let service = &state.services.custom_service_definition_service;
    let existing = service
        .get_by_id(&path.0)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<CustomServiceDefinition>(&path.0))?;
    if existing.base.organization_id != Some(organization_id) {
        return Err(ApiError::entity_not_found::<CustomServiceDefinition>(
            &path.0,
        ));
    }

    guard_custom_service_deletion(service.storage().pool(), &organization_id, &existing.id).await?;

    delete_handler::<CustomServiceDefinition>(State(state), auth, path).await
}

/// The merged service catalogue: built-in definitions (read-only) followed by
/// the caller's organization's custom definitions (full CRUD). Single merge
/// point, owned by the backend.
#[utoipa::path(
    get,
    path = "",
    tag = "Service Catalogue",
    responses(
        (status = 200, description = "Merged built-in + custom service catalogue", body = Vec<ServiceCatalogueEntry>),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_service_catalogue(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
) -> ApiResult<Json<Vec<ServiceCatalogueEntry>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    let mut entries: Vec<ServiceCatalogueEntry> = Vec::new();

    for definition in ServiceDefinitionRegistry::all_service_definitions() {
        let category = ServiceDefinition::category(&definition);
        entries.push(ServiceCatalogueEntry {
            kind: ServiceCatalogueEntryKind::BuiltIn,
            id: definition.id().to_string(),
            name: ServiceDefinition::name(&definition).to_string(),
            description: ServiceDefinition::description(&definition).to_string(),
            category: category.id().to_string(),
            color: Some(EntityMetadataProvider::color(&definition).to_string()),
            icon: Some(EntityMetadataProvider::icon(&definition).to_string()),
            logo_url: ServiceDefinition::logo_url(&definition).to_string(),
            logo_needs_white_background: ServiceDefinition::logo_needs_white_background(
                &definition,
            ),
            is_generic: ServiceDefinition::is_generic(&definition),
            custom_id: None,
        });
    }

    // Custom entries are tenant-scoped: only the caller's org's rows are merged.
    let filter = StorableFilter::<CustomServiceDefinition>::new_from_org_id(&organization_id);
    let customs = state
        .services
        .custom_service_definition_service
        .get_all(filter)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    for custom in customs {
        let category_color = ServiceCategory::iter()
            .find(|category| category.id() == custom.base.category)
            .map(|category| category.color().to_string());
        let category_icon = ServiceCategory::iter()
            .find(|category| category.id() == custom.base.category)
            .map(|category| category.icon().to_string());
        entries.push(ServiceCatalogueEntry {
            kind: ServiceCatalogueEntryKind::Custom,
            id: custom.base.name.clone(),
            name: custom.base.name,
            description: custom.base.description,
            category: custom.base.category,
            color: category_color,
            icon: category_icon,
            logo_url: custom.base.logo_url,
            logo_needs_white_background: custom.base.logo_needs_white_background,
            is_generic: custom.base.is_generic,
            custom_id: Some(custom.id),
        });
    }

    Ok(Json(entries))
}

/// Block deletion when a custom service row is still referenced by a host
/// override in the caller's organization.
///
/// INVARIANT 5 defence: the `host_port_overrides` table is created by the #10
/// work (PonyTailGLM) in parallel and carries `service_ref_kind` /
/// `service_ref_id` with NO foreign key (built-ins are compile-time, so no FK
/// can span both namespaces). If the table does not exist in this tree yet,
/// the `to_regclass` guard skips the check so nothing breaks; once it lands at
/// integration, the guard resolves to the real table automatically.
pub(crate) async fn guard_custom_service_deletion(
    pool: &sqlx::PgPool,
    organization_id: &Uuid,
    custom_id: &Uuid,
) -> Result<(), ApiError> {
    let table_exists: bool =
        sqlx::query_scalar("SELECT to_regclass('public.host_port_overrides') IS NOT NULL")
            .fetch_one(pool)
            .await
            .map_err(|error| ApiError::internal_error(&error.to_string()))?;
    if !table_exists {
        return Ok(());
    }

    let referencing_hosts: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT h.name
         FROM host_port_overrides o
         JOIN hosts h ON h.id = o.host_id
         JOIN networks n ON n.id = h.network_id
         WHERE o.service_ref_kind = 'Custom'
           AND o.service_ref_id = $1
           AND n.organization_id = $2
         ORDER BY h.name",
    )
    .bind(custom_id.to_string())
    .bind(organization_id)
    .fetch_all(pool)
    .await
    .map_err(|error| ApiError::internal_error(&error.to_string()))?;

    if !referencing_hosts.is_empty() {
        return Err(ApiError::conflict(&format!(
            "This custom service is referenced by host overrides on {} and cannot be deleted. \
             Remove or reassign those overrides first: {}",
            referencing_hosts.len(),
            referencing_hosts.join(", ")
        )));
    }

    Ok(())
}

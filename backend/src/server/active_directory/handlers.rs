use std::sync::Arc;

use axum::{
    Json,
    extract::{DefaultBodyLimit, State},
};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::server::{
    auth::middleware::permissions::{Authorized, IsDaemon, Viewer},
    config::AppState,
    credentials::r#impl::{mapping::IntegrationTarget, types::CredentialType},
    shared::{
        extractors::Query,
        services::traits::CrudService,
        types::api::{ApiError, ApiErrorResponse, ApiResponse, ApiResult, PaginatedApiResponse},
    },
};

use super::{
    storage,
    types::{
        AdCollectionRequest, AdCollectionRun, AdDomain, AdEntity, AdEntityListQuery, AdListQuery,
        MAX_REQUEST_BODY_BYTES,
    },
};

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_domains))
        .routes(routes!(get_entities))
        .routes(routes!(get_collection_runs, ingest_collection))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BODY_BYTES))
}

/// List normalized Active Directory domains visible to the caller.
#[utoipa::path(
    get,
    path = "/domains",
    tag = "Active Directory",
    params(AdListQuery),
    responses(
        (status = 200, description = "Active Directory domains", body = PaginatedApiResponse<AdDomain>)
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_domains(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Query(query): Query<AdListQuery>,
) -> ApiResult<Json<PaginatedApiResponse<AdDomain>>> {
    let network_ids = auth.network_ids();
    let (items, total) = storage::list_domains(&state.pool, &network_ids, &query)
        .await
        .map_err(internal_storage_error)?;
    let (limit, offset) = query.pagination();
    Ok(Json(PaginatedApiResponse::success(
        items, total, limit, offset,
    )))
}

/// List normalized Active Directory inventory/topology entities.
#[utoipa::path(
    get,
    path = "/entities",
    tag = "Active Directory",
    params(AdEntityListQuery),
    responses(
        (status = 200, description = "Active Directory entities", body = PaginatedApiResponse<AdEntity>)
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_entities(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Query(query): Query<AdEntityListQuery>,
) -> ApiResult<Json<PaginatedApiResponse<AdEntity>>> {
    let network_ids = auth.network_ids();
    let (items, total) = storage::list_entities(&state.pool, &network_ids, &query)
        .await
        .map_err(internal_storage_error)?;
    let (limit, offset) = query.pagination();
    Ok(Json(PaginatedApiResponse::success(
        items, total, limit, offset,
    )))
}

/// List bounded Active Directory collection provenance and issue summaries.
#[utoipa::path(
    get,
    path = "/collection-runs",
    tag = "Active Directory",
    params(AdListQuery),
    responses(
        (status = 200, description = "Active Directory collection runs", body = PaginatedApiResponse<AdCollectionRun>)
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_collection_runs(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Query(query): Query<AdListQuery>,
) -> ApiResult<Json<PaginatedApiResponse<AdCollectionRun>>> {
    let network_ids = auth.network_ids();
    let (items, total) = storage::list_collection_runs(&state.pool, &network_ids, &query)
        .await
        .map_err(internal_storage_error)?;
    let (limit, offset) = query.pagination();
    Ok(Json(PaginatedApiResponse::success(
        items, total, limit, offset,
    )))
}

/// Atomically ingest one daemon Active Directory collection result.
///
/// Only a complete successful result replaces prior inventory. Credential,
/// target, discovery, and session identity are re-authorized server-side.
#[utoipa::path(
    post,
    path = "/collection-runs",
    tag = "Active Directory",
    request_body = AdCollectionRequest,
    responses(
        (status = 200, description = "Collection persisted", body = ApiResponse<AdCollectionRun>),
        (status = 400, description = "Invalid or over-limit collection", body = ApiErrorResponse),
        (status = 403, description = "Daemon is not assigned to the network", body = ApiErrorResponse),
        (status = 404, description = "Network not found", body = ApiErrorResponse)
    ),
    security(("daemon_api_key" = []))
)]
async fn ingest_collection(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsDaemon>,
    Json(request): Json<AdCollectionRequest>,
) -> ApiResult<Json<ApiResponse<AdCollectionRun>>> {
    request
        .validate()
        .map_err(|message| ApiError::bad_request(&message))?;

    if !auth.network_ids().contains(&request.network_id) {
        return Err(ApiError::forbidden(
            "Daemon cannot persist Active Directory inventory for an unassigned network",
        ));
    }
    let daemon_id = auth
        .daemon_id()
        .ok_or_else(|| ApiError::forbidden("Daemon identity is required"))?;
    let session = state
        .services
        .discovery_service
        .get_session(&request.session_id)
        .await
        .ok_or_else(|| ApiError::forbidden("AD collection session is not active"))?;
    if session.daemon_id != daemon_id
        || session.network_id != request.network_id
        || session.discovery_id != Some(request.discovery_id)
        || session.phase.is_terminal()
    {
        return Err(ApiError::forbidden(
            "AD collection session does not match this daemon and discovery",
        ));
    }

    let organization_id = storage::organization_for_network(&state.pool, request.network_id)
        .await
        .map_err(internal_storage_error)?
        .ok_or_else(|| ApiError::not_found("Network not found".to_string()))?;

    let discovery = state
        .services
        .discovery_service
        .get_by_id(&request.discovery_id)
        .await?
        .ok_or_else(|| ApiError::forbidden("AD collection discovery is not available"))?;
    if discovery.base.daemon_id != daemon_id || discovery.base.network_id != request.network_id {
        return Err(ApiError::forbidden(
            "AD collection discovery is not assigned to this daemon and network",
        ));
    }

    let credential = state
        .services
        .credential_service
        .get_by_id(&request.credential_id)
        .await?
        .filter(|credential| credential.base.organization_id == organization_id)
        .ok_or_else(|| ApiError::forbidden("AD credential target is not authorized"))?;
    let collector = match &credential.base.credential_type {
        CredentialType::ActiveDirectoryLdaps { .. } => super::types::AdCollector::Ldaps,
        CredentialType::ActiveDirectoryKerberos { .. } => super::types::AdCollector::Kerberos,
        _ => {
            return Err(ApiError::forbidden(
                "Credential is not an Active Directory collection credential",
            ));
        }
    };

    let discovery_target_authorized = integration_target_authorizes(
        &discovery.integration_targets,
        request.credential_id,
        request.target_ip,
    );
    let host_assignment_authorized = storage::host_assignment_authorizes(
        &state.pool,
        request.network_id,
        request.target_host_id,
        request.target_ip,
        request.credential_id,
    )
    .await
    .map_err(internal_storage_error)?;
    if !discovery_target_authorized && !host_assignment_authorized {
        return Err(ApiError::forbidden(
            "AD credential is not assigned to this target host and IP",
        ));
    }

    let live_hosts =
        storage::live_hosts_for_target_ip(&state.pool, request.network_id, request.target_ip)
            .await
            .map_err(internal_storage_error)?;
    if !exact_live_host_matches(&live_hosts, request.target_host_id) {
        return Err(ApiError::forbidden(
            "AD target must resolve to exactly the assigned live host and IP",
        ));
    }

    let verified_target = storage::VerifiedAdTarget {
        organization_id,
        network_id: request.network_id,
        daemon_id,
        credential_id: request.credential_id,
        target_host_id: request.target_host_id,
        target_ip: request.target_ip,
        discovery_id: request.discovery_id,
        session_id: request.session_id,
        collection_key: format!(
            "{}@{}@{}",
            request.credential_id, request.target_host_id, request.target_ip
        ),
        collector,
    };
    let run = storage::ingest_collection(&state.pool, &verified_target, &request)
        .await
        .map_err(internal_storage_error)?;
    Ok(Json(ApiResponse::success(run)))
}

fn exact_live_host_matches(live_hosts: &[uuid::Uuid], requested_host_id: uuid::Uuid) -> bool {
    live_hosts == [requested_host_id]
}

fn integration_target_authorizes(
    targets: &[IntegrationTarget],
    credential_id: uuid::Uuid,
    target_ip: std::net::IpAddr,
) -> bool {
    targets.iter().any(|target| {
        matches!(
            target,
            IntegrationTarget::Hosts {
                credential_id: assigned_credential_id,
                ips,
            } if *assigned_credential_id == credential_id && ips.contains(&target_ip)
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn integration_target_requires_exact_credential_and_ip() {
        let credential_id = Uuid::new_v4();
        let ip = "192.0.2.10".parse().unwrap();
        let targets = vec![IntegrationTarget::Hosts {
            credential_id,
            ips: vec![ip],
        }];
        assert!(integration_target_authorizes(&targets, credential_id, ip));
        assert!(!integration_target_authorizes(&targets, Uuid::new_v4(), ip));
        assert!(!integration_target_authorizes(
            &targets,
            credential_id,
            "192.0.2.11".parse().unwrap()
        ));
    }

    #[test]
    fn target_requires_exactly_one_matching_live_host() {
        let host_id = Uuid::new_v4();
        assert!(exact_live_host_matches(&[host_id], host_id));
        assert!(!exact_live_host_matches(&[], host_id));
        assert!(!exact_live_host_matches(&[Uuid::new_v4()], host_id));
        assert!(!exact_live_host_matches(
            &[host_id, Uuid::new_v4()],
            host_id
        ));
    }
}

fn internal_storage_error(error: anyhow::Error) -> ApiError {
    tracing::error!(error = %error, "Active Directory persistence failed");
    ApiError::internal_error("Active Directory persistence failed")
}

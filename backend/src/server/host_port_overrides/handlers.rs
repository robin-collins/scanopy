use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
use validator::Validate;

use crate::server::auth::middleware::permissions::{Authorized, Member};
use crate::server::config::AppState;
use crate::server::host_port_overrides::{
    r#impl::base::{HostPortOverride, HostPortOverrideBase, ServiceRefKind},
    service::HostPortOverrideService,
};
use crate::server::hosts::r#impl::base::Host;
use crate::server::shared::handlers::query::HostChildQuery;
use crate::server::shared::handlers::traits::CrudHandlers;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::traits::Entity;
use crate::server::shared::types::api::{
    ApiError, ApiErrorResponse, ApiResponse, ApiResult, EmptyApiResponse,
};

impl CrudHandlers for HostPortOverride {
    type Service = HostPortOverrideService;
    type FilterQuery = HostChildQuery;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.host_port_override_service
    }
}

mod generated {
    use super::*;
    crate::crud_get_all_handler!(HostPortOverride);
    crate::crud_get_by_id_handler!(HostPortOverride);
}

/// Incoming upsert payload. `network_id` is deliberately NOT accepted — the
/// backend derives it from the host (the backend owns merging/validation).
/// Service-ref kind + id are validated here (format per kind) so the
/// discriminator and the id can never disagree.
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct HostPortOverrideInput {
    /// The host this override applies to.
    pub host_id: Uuid,
    /// Port number this override applies to.
    #[validate(range(min = 0, max = 65535))]
    #[schema(minimum = 0, maximum = 65535)]
    pub port_number: i64,
    /// Transport protocol this override applies to. One of `Tcp`/`Udp`.
    pub port_protocol: String,
    /// Per-host display/service name. NULL = fall back to the global default.
    #[validate(length(max = 255))]
    pub display_name: Option<String>,
    /// Per-host icon URL. NULL = use the default icon.
    #[validate(length(max = 2048))]
    pub icon_url: Option<String>,
    /// Catalogue-reference discriminator, when assigning a service.
    pub service_ref_kind: Option<ServiceRefKind>,
    /// Built-in ServiceDefinition id OR custom row UUID, depending on kind.
    pub service_ref_id: Option<String>,
}

fn validate_input(input: &HostPortOverrideInput) -> Result<(), ApiError> {
    if input.port_protocol != "Tcp" && input.port_protocol != "Udp" {
        return Err(ApiError::bad_request(
            "port_protocol must be one of 'Tcp' or 'Udp'",
        ));
    }
    input
        .validate()
        .map_err(|e| ApiError::bad_request(&format!("Invalid override: {e}")))?;

    match (input.service_ref_kind, input.service_ref_id.as_deref()) {
        (None, None) => {}
        (None, Some(_)) | (Some(_), None) => {
            return Err(ApiError::bad_request(
                "service_ref_kind and service_ref_id must be both set or both omitted",
            ));
        }
        (Some(ServiceRefKind::BuiltIn), Some(id)) => {
            if Uuid::parse_str(id).is_ok() {
                return Err(ApiError::bad_request(
                    "a BuiltIn service reference id must not be a UUID",
                ));
            }
        }
        (Some(ServiceRefKind::Custom), Some(id)) => {
            if Uuid::parse_str(id).is_err() {
                return Err(ApiError::bad_request(
                    "a Custom service reference id must be a valid UUID",
                ));
            }
        }
    }
    Ok(())
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(generated::get_all, upsert))
        .routes(routes!(generated::get_by_id))
        .routes(routes!(clear))
}

/// Upsert a single per-host port override. The backend resolves `network_id`
/// from the host and validates every field, including the tagged-union
/// catalogue reference.
#[utoipa::path(
    put,
    path = "",
    tag = HostPortOverride::ENTITY_NAME_PLURAL,
    responses(
        (status = 200, description = "Override upserted", body = ApiResponse<HostPortOverride>),
        (status = 404, description = "Host not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn upsert(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Json(input): Json<HostPortOverrideInput>,
) -> ApiResult<Json<ApiResponse<HostPortOverride>>> {
    validate_input(&input)?;

    let host = state
        .services
        .host_service
        .get_by_id(&input.host_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Host>(input.host_id))?;
    if !auth.network_ids().contains(&host.base.network_id) {
        return Err(ApiError::entity_not_found::<Host>(input.host_id));
    }

    let base = HostPortOverrideBase {
        host_id: input.host_id,
        network_id: host.base.network_id,
        port_number: input.port_number,
        port_protocol: input.port_protocol,
        display_name: input.display_name,
        icon_url: input.icon_url,
        service_ref_kind: input.service_ref_kind,
        service_ref_id: input.service_ref_id,
    };

    let saved = state
        .services
        .host_port_override_service
        .upsert(base, auth.entity.clone())
        .await?;

    Ok(Json(ApiResponse::success(saved)))
}

/// Remove a per-host port override (reset to the global default).
#[utoipa::path(
    delete,
    path = "/{host_id}/{port_number}/{port_protocol}",
    tag = HostPortOverride::ENTITY_NAME_PLURAL,
    responses(
        (status = 200, description = "Override removed", body = EmptyApiResponse),
        (status = 404, description = "Host not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn clear(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path((host_id, port_number, port_protocol)): Path<(Uuid, i64, String)>,
) -> ApiResult<Json<EmptyApiResponse>> {
    if port_protocol != "Tcp" && port_protocol != "Udp" {
        return Err(ApiError::bad_request(
            "port_protocol must be one of 'Tcp' or 'Udp'",
        ));
    }

    let removed = state
        .services
        .host_port_override_service
        .clear(&host_id, port_number, &port_protocol, auth.entity.clone())
        .await?;
    if !removed {
        return Err(ApiError::not_found(
            "No override exists for this port".to_string(),
        ));
    }
    Ok(Json(ApiResponse::success(())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(kind: Option<ServiceRefKind>, id: Option<&str>) -> HostPortOverrideInput {
        HostPortOverrideInput {
            host_id: Uuid::new_v4(),
            port_number: 443,
            port_protocol: "Tcp".to_string(),
            display_name: None,
            icon_url: None,
            service_ref_kind: kind,
            service_ref_id: id.map(str::to_string),
        }
    }

    #[test]
    fn service_reference_must_be_both_set_or_both_omitted() {
        assert!(validate_input(&input(None, None)).is_ok());
        assert!(validate_input(&input(None, Some("HTTP Server"))).is_err());
        assert!(validate_input(&input(Some(ServiceRefKind::BuiltIn), None)).is_err());
    }

    #[test]
    fn service_reference_kind_must_match_id_shape() {
        let uuid = Uuid::new_v4().to_string();
        assert!(
            validate_input(&input(Some(ServiceRefKind::BuiltIn), Some(&uuid))).is_err(),
            "a UUID cannot identify a built-in service"
        );
        assert!(
            validate_input(&input(Some(ServiceRefKind::Custom), Some("HTTP Server"))).is_err(),
            "a non-UUID cannot identify a custom service"
        );
        assert!(validate_input(&input(Some(ServiceRefKind::BuiltIn), Some("HTTP Server"))).is_ok());
        assert!(validate_input(&input(Some(ServiceRefKind::Custom), Some(&uuid))).is_ok());
    }

    #[test]
    fn port_number_must_fit_the_network_port_range() {
        let mut candidate = input(None, None);
        candidate.port_number = -1;
        assert!(validate_input(&candidate).is_err());
        candidate.port_number = 65_536;
        assert!(validate_input(&candidate).is_err());
        candidate.port_number = 65_535;
        assert!(validate_input(&candidate).is_ok());
    }
}

use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::auth::middleware::permissions::{Authorized, IsUser, Member, Owner};
use crate::server::auth::service::hash_password;
use crate::server::billing::types::base::BillingPlan;
use crate::server::bindings::r#impl::base::Binding;
use crate::server::config::AppState;
use crate::server::networks::r#impl::{Network, NetworkBase};
use crate::server::organizations::r#impl::base::Organization;
use crate::server::shared::events::types::{OnboardingEvent, OnboardingOperation};
use crate::server::shared::handlers::traits::{CrudHandlers, update_handler};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::{Entity, Storable, Storage};
use crate::server::shared::types::api::ApiResponse;
use crate::server::shared::types::api::ApiResult;
use crate::server::shared::types::api::{ApiError, ApiErrorResponse, EmptyApiResponse};
use crate::server::users::r#impl::base::{User, UserBase};
use crate::server::users::r#impl::permissions::UserOrgPermissions;
use anyhow::anyhow;
use axum::Json;
use axum::extract::Path;
use axum::extract::State;
use chrono::Utc;
use email_address::EmailAddress;
use serde::Deserialize;
use std::sync::Arc;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

pub const DEMO_USER_ID: Uuid = Uuid::from_u128(0x550e8400_e29b_41d4_a716_446655440050);

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_organization, update_org_name))
        .routes(routes!(update_profile))
        .routes(routes!(submit_referral_source))
        .routes(routes!(reset))
        .routes(routes!(populate_demo_data))
}

/// Get the current user's organization
#[utoipa::path(
    get,
    path = "",
    tag = Organization::ENTITY_NAME_PLURAL,
    responses(
        (status = 200, description = "Organization details", body = ApiResponse<Organization>),
        (status = 404, description = "Organization not found", body = ApiErrorResponse),
    ),
    security(("session" = []))
)]
pub async fn get_organization(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsUser>,
) -> ApiResult<Json<ApiResponse<Organization>>> {
    let organization_id = auth.require_organization_id()?;
    let service = Organization::get_service(&state);
    let entity = service
        .get_by_id(&organization_id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?
        .ok_or_else(|| ApiError::entity_not_found::<Organization>(organization_id))?;

    Ok(Json(ApiResponse::success(entity)))
}

/// Update organization name
#[utoipa::path(
    put,
    path = "/{id}",
    tag = Organization::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Organization ID")),
    request_body = String,
    responses(
        (status = 200, description = "Organization updated", body = ApiResponse<Organization>),
        (status = 403, description = "Only owners can update organization", body = ApiErrorResponse),
        (status = 404, description = "Organization not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn update_org_name(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Path(id): Path<Uuid>,
    Json(name): Json<String>,
) -> ApiResult<Json<ApiResponse<Organization>>> {
    let mut org = state
        .services
        .organization_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| anyhow!("Could not find org"))?;

    org.base.name = name;

    update_handler::<Organization>(
        axum::extract::State(state),
        auth.into_permission::<Member>(),
        axum::extract::Path(id),
        axum::extract::Json(org),
    )
    .await
}

/// Request to update user profile (deferred marketing fields)
#[derive(Debug, Deserialize, ToSchema)]
pub struct ProfileUpdateRequest {
    pub job_title: Option<String>,
    pub company_size: Option<String>,
}

/// Update user profile with deferred marketing fields
#[utoipa::path(
    post,
    path = "/profile",
    tag = Organization::ENTITY_NAME_PLURAL,
    request_body = ProfileUpdateRequest,
    responses(
        (status = 200, description = "Profile updated", body = EmptyApiResponse),
    )
)]
async fn update_profile(
    auth: Authorized<IsUser>,
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProfileUpdateRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let org_id = auth.organization_id().unwrap();
    let authentication: AuthenticatedEntity = auth.into();

    state
        .services
        .event_bus
        .publish_onboarding(OnboardingEvent {
            id: Uuid::new_v4(),
            organization_id: org_id,
            operation: OnboardingOperation::ProfileCompleted,
            timestamp: Utc::now(),
            authentication,
            metadata: serde_json::json!({
                "job_title": request.job_title,
                "company_size": request.company_size,
            }),
        })
        .await
        .map_err(|e| {
            ApiError::internal_error(&format!("Failed to publish profile event: {}", e))
        })?;

    Ok(Json(ApiResponse::success(())))
}

/// Request to submit referral source
#[derive(Debug, Deserialize, ToSchema)]
pub struct ReferralSourceRequest {
    pub referral_source: String,
    pub referral_source_other: Option<String>,
}

/// Submit referral source (how did you hear about us)
#[utoipa::path(
    post,
    path = "/referral-source",
    tag = Organization::ENTITY_NAME_PLURAL,
    request_body = ReferralSourceRequest,
    responses(
        (status = 200, description = "Referral source recorded", body = EmptyApiResponse),
    )
)]
async fn submit_referral_source(
    auth: Authorized<IsUser>,
    State(state): State<Arc<AppState>>,
    Json(request): Json<ReferralSourceRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let org_id = auth.organization_id().unwrap();
    let authentication: AuthenticatedEntity = auth.into();

    state
        .services
        .event_bus
        .publish_onboarding(OnboardingEvent {
            id: Uuid::new_v4(),
            organization_id: org_id,
            operation: OnboardingOperation::ReferralSourceCompleted,
            timestamp: Utc::now(),
            authentication,
            metadata: serde_json::json!({
                "referral_source": request.referral_source,
                "referral_source_other": request.referral_source_other,
            }),
        })
        .await
        .map_err(|e| {
            ApiError::internal_error(&format!("Failed to publish referral source event: {}", e))
        })?;

    Ok(Json(ApiResponse::success(())))
}

/// Reset all organization data (delete all entities except organization and owner user)
#[utoipa::path(
    post,
    path = "/{id}/reset",
    tags = [Organization::ENTITY_NAME_PLURAL, "internal"],
    params(("id" = Uuid, Path, description = "Organization ID")),
    responses(
        (status = 200, description = "Organization reset", body = EmptyApiResponse),
        (status = 403, description = "Cannot reset another organization", body = ApiErrorResponse),
        (status = 404, description = "Organization not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn reset(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let user_org_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    // Verify organization exists
    let org = state
        .services
        .organization_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Organization>(id))?;

    if org.id != user_org_id {
        return Err(ApiError::permission_denied());
    }

    let entity: AuthenticatedEntity = auth.into_entity();

    reset_organization_data(&state, &org.id, entity.clone()).await?;

    // Create a default network so the org always has at least one
    let network = Network::new(NetworkBase::new(org.id));
    let network = state
        .services
        .network_service
        .create(network, entity.clone())
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to create network: {}", e)))?;

    state
        .services
        .network_service
        .create_organizational_subnets(network.id, entity)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to seed data: {}", e)))?;

    Ok(Json(ApiResponse::success(())))
}

/// Populate demo data (only available for demo organizations)
#[utoipa::path(
    post,
    path = "/{id}/populate-demo",
    tags = [Organization::ENTITY_NAME_PLURAL, "internal"],
    params(("id" = Uuid, Path, description = "Organization ID")),
    responses(
        (status = 200, description = "Demo data populated", body = EmptyApiResponse),
        (status = 403, description = "Only available for demo organizations", body = ApiErrorResponse),
        (status = 404, description = "Organization not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn populate_demo_data(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    use crate::server::organizations::demo_data::{DemoData, generate_groups};
    use crate::server::services::r#impl::base::Service;

    let user_org_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let user_id = auth.user_id().ok_or_else(ApiError::user_required)?;

    let mut org = state
        .services
        .organization_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Organization>(id))?;

    if org.id != user_org_id {
        return Err(ApiError::permission_denied());
    }

    // Only available for demo organizations
    if !matches!(org.base.plan, Some(BillingPlan::Demo(_))) {
        return Err(ApiError::forbidden(
            "Populate demo data is only available for demo organizations",
        ));
    }

    let entity: AuthenticatedEntity = auth.into_entity();

    // First, reset all existing data
    reset_organization_data(&state, &id, entity.clone()).await?;

    org.base.onboarding = vec![
        OnboardingOperation::OrgCreated,
        OnboardingOperation::FirstDaemonRegistered,
        OnboardingOperation::FirstDiscoveryCompleted,
    ];

    state
        .services
        .organization_service
        .update(&mut org, entity.clone())
        .await?;

    // Generate demo data
    let demo_data = DemoData::generate(id, user_id);

    // Helper to bulk-set tags for entities that have them.
    // Tags live in a junction table, so they must be set after the entity rows exist.
    async fn set_tags_for_entities<T: Entity>(
        entity_tag_service: &crate::server::tags::entity_tags::EntityTagService,
        entities: &[T],
        org_id: Uuid,
    ) -> Result<(), ApiError> {
        for entity in entities {
            if let Some(tags) = entity.get_tags()
                && !tags.is_empty()
            {
                entity_tag_service
                    .set_tags(entity.id(), T::entity_type(), tags.clone(), org_id)
                    .await
                    .map_err(|e| ApiError::internal_error(&e.to_string()))?;
            }
        }
        Ok(())
    }

    // Insert entities in dependency order using bulk inserts.
    // Since we just reset the org, there are no collisions — we use service-level
    // create_many (publishes one event per scope per entity type) instead of
    // per-entity create() for speed. Entities without event subscribers use
    // storage().create_many() directly.

    // 1. Tags (no dependencies)
    let created_tags = state
        .services
        .tag_service
        .storage()
        .create_many(&demo_data.tags)
        .await?;

    // 2. Credentials (depends on organization)
    state
        .services
        .credential_service
        .storage()
        .create_many(&demo_data.credentials)
        .await?;

    // 3. Networks (depends on organization, tags)
    let created_networks = state
        .services
        .network_service
        .storage()
        .create_many(&demo_data.networks)
        .await?;
    set_tags_for_entities(&state.services.entity_tag_service, &created_networks, id).await?;

    // 3.5. Network-credential associations
    for assignment in demo_data.network_credential_assignments {
        state
            .services
            .credential_service
            .set_network_credentials(&assignment.network_id, &assignment.credential_ids)
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    }

    // 4. Subnets (depends on networks)
    let created_subnets = state
        .services
        .subnet_service
        .create_many(&demo_data.subnets, entity.clone())
        .await?;
    set_tags_for_entities(&state.services.entity_tag_service, &created_subnets, id).await?;

    // 5. Hosts + children — bypass discover_host (no collisions in fresh org)
    // Flatten hosts, interfaces, ports, services from HostWithServices bundles
    let mut all_hosts = Vec::new();
    let mut all_interfaces = Vec::new();
    let mut all_ports = Vec::new();
    let mut all_services: Vec<Service> = Vec::new();
    for hws in &demo_data.hosts_with_services {
        let host_id = hws.host.id;
        let network_id = hws.host.base.network_id;
        all_hosts.push(hws.host.clone());
        all_interfaces.extend(hws.interfaces.clone());
        all_ports.extend(
            hws.ports
                .iter()
                .cloned()
                .map(|p| p.with_host(host_id, network_id)),
        );
        all_services.extend(hws.services.clone());
    }

    let created_hosts = state
        .services
        .host_service
        .create_many(&all_hosts, entity.clone())
        .await?;
    set_tags_for_entities(&state.services.entity_tag_service, &created_hosts, id).await?;

    state
        .services
        .interface_service
        .create_many(&all_interfaces, entity.clone())
        .await?;

    state
        .services
        .port_service
        .create_many(&all_ports, entity.clone())
        .await?;

    let created_services = state
        .services
        .service_service
        .create_many(&all_services, entity.clone())
        .await?;
    set_tags_for_entities(&state.services.entity_tag_service, &created_services, id).await?;

    // 5.3. Bindings (child entities of services, stored in separate table)
    let all_bindings: Vec<Binding> = created_services
        .iter()
        .flat_map(|s| {
            s.base
                .bindings
                .iter()
                .cloned()
                .map(|b| b.with_service(s.id, s.base.network_id))
        })
        .collect();
    state
        .services
        .binding_service
        .create_many(&all_bindings, entity.clone())
        .await?;

    // 5.5. IfEntries (depends on hosts)
    state
        .services
        .if_entry_service
        .create_many(&demo_data.if_entries, entity.clone())
        .await?;

    // 5.6. Apply neighbor updates — IDs are preserved from demo data, so build
    // the lookup directly from what we inserted (no DB round-trip needed)
    use crate::server::if_entries::r#impl::base::Neighbor;
    use std::collections::HashMap;

    let host_id_to_name: HashMap<Uuid, String> = all_hosts
        .iter()
        .map(|h| (h.id, h.base.name.clone()))
        .collect();

    let mut if_entry_lookup: HashMap<
        (String, i32),
        &crate::server::if_entries::r#impl::base::IfEntry,
    > = HashMap::new();
    for entry in &demo_data.if_entries {
        if let Some(host_name) = host_id_to_name.get(&entry.base.host_id) {
            if_entry_lookup.insert((host_name.clone(), entry.base.if_index), entry);
        }
    }

    for neighbor_update in &demo_data.neighbor_updates {
        let source_key = (
            neighbor_update.source_host_name.clone(),
            neighbor_update.source_if_index,
        );
        let target_key = (
            neighbor_update.target_host_name.clone(),
            neighbor_update.target_if_index,
        );

        if let (Some(source_entry), Some(target_entry)) = (
            if_entry_lookup.get(&source_key),
            if_entry_lookup.get(&target_key),
        ) {
            let mut updated_entry = (*source_entry).clone();
            updated_entry.base.neighbor = Some(Neighbor::IfEntry(target_entry.id));
            state
                .services
                .if_entry_service
                .storage()
                .update(&mut updated_entry)
                .await?;
        }
    }

    // 6. Daemons (depends on hosts, networks, subnets)
    state
        .services
        .daemon_service
        .storage()
        .create_many(&demo_data.daemons)
        .await?;

    // 7. Daemon API Keys (depends on networks)
    state
        .services
        .daemon_api_key_service
        .storage()
        .create_many(&demo_data.api_keys)
        .await?;

    // 8. Discoveries (depends on daemons, networks, subnets)
    state
        .services
        .discovery_service
        .storage()
        .create_many(&demo_data.discoveries)
        .await?;

    // 9. Groups — generate with created services to get correct binding IDs
    let groups = generate_groups(&created_networks, &created_services, &created_tags);
    state
        .services
        .group_service
        .create_many(&groups, entity.clone())
        .await?;

    // 10. Topologies (depends on networks)
    state
        .services
        .topology_service
        .storage()
        .create_many(&demo_data.topologies)
        .await?;

    // 11. Shares (depends on topologies)
    state
        .services
        .share_service
        .storage()
        .create_many(&demo_data.shares)
        .await?;

    // 12. Demo admin user
    let password = hash_password("password123")?;
    let mut demo_admin = User::new(UserBase::new_password(
        EmailAddress::new_unchecked("demo@scanopy.net"),
        password,
        org.id,
        UserOrgPermissions::Admin,
        vec![],
        None,
    ));
    demo_admin.base.email_verified = true;
    demo_admin.id = DEMO_USER_ID;
    state
        .services
        .user_service
        .create(demo_admin, entity.clone())
        .await?;

    // 13. User API Keys (depends on demo admin user + network access junction table)
    for (api_key, network_ids) in demo_data.user_api_keys {
        state
            .services
            .user_api_key_service
            .create_with_networks(api_key, network_ids, entity.clone())
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    }

    Ok(Json(ApiResponse::success(())))
}

/// Internal function to reset organization data (reused by populate_demo_data)
async fn reset_organization_data(
    state: &Arc<AppState>,
    organization_id: &Uuid,
    auth: AuthenticatedEntity,
) -> Result<(), ApiError> {
    let org_filter = StorableFilter::<Network>::new_from_org_id(organization_id);
    let network_ids: Vec<Uuid> = state
        .services
        .network_service
        .get_all(org_filter.clone())
        .await?
        .iter()
        .map(|n| n.id)
        .collect();

    // Delete all data except org and owner user
    // Order matters due to foreign keys:
    // 1. Shares depend on topologies/networks
    // 2. Discoveries depend on daemons/networks
    // 3. Daemons depend on hosts/networks
    // 4. Hosts/services depend on networks
    // 5. Topologies depend on networks
    // 6. API keys (daemon + user) depend on networks/users
    // 7. Networks, credentials, tags, invites
    state
        .services
        .share_service
        .delete_all_for_org(organization_id, &network_ids, auth.clone())
        .await?;
    state
        .services
        .discovery_service
        .delete_all_for_org(organization_id, &network_ids, auth.clone())
        .await?;
    state
        .services
        .daemon_service
        .delete_all_for_org(organization_id, &network_ids, auth.clone())
        .await?;
    state
        .services
        .host_service
        .delete_all_for_org(organization_id, &network_ids, auth.clone())
        .await?;
    state
        .services
        .topology_service
        .delete_all_for_org(organization_id, &network_ids, auth.clone())
        .await?;
    state
        .services
        .daemon_api_key_service
        .delete_all_for_org(organization_id, &network_ids, auth.clone())
        .await?;
    state
        .services
        .user_api_key_service
        .delete_all_for_org(organization_id, &network_ids, auth.clone())
        .await?;
    state
        .services
        .network_service
        .delete_all_for_org(organization_id, &network_ids, auth.clone())
        .await?;
    state
        .services
        .invite_service
        .delete_all_for_org(organization_id, &network_ids, auth.clone())
        .await?;
    state
        .services
        .credential_service
        .delete_all_for_org(organization_id, &network_ids, auth.clone())
        .await?;
    state
        .services
        .tag_service
        .delete_all_for_org(organization_id, &network_ids, auth.clone())
        .await?;

    // Delete non-owner users
    let user_filter = StorableFilter::<User>::new_from_org_id(organization_id);
    let non_owner_user_ids: Vec<Uuid> = state
        .services
        .user_service
        .get_all(user_filter)
        .await?
        .iter()
        .filter_map(|u| {
            if u.base.permissions != UserOrgPermissions::Owner {
                Some(u.id)
            } else {
                None
            }
        })
        .collect();

    state
        .services
        .user_service
        .delete_many(&non_owner_user_ids, auth)
        .await?;

    Ok(())
}

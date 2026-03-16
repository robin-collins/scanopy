use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    hosts::{r#impl::base::Host, service::HostService},
    interfaces::{r#impl::base::Interface, service::InterfaceService},
    networks::service::NetworkService,
    organizations::service::OrganizationService,
    shared::{
        events::{
            bus::EventBus,
            types::{OnboardingEvent, OnboardingOperation},
        },
        services::traits::{CrudService, EventBusService},
        storage::{filter::StorableFilter, generic::GenericPostgresStorage},
    },
    snmp_credentials::r#impl::{
        base::SnmpCredential,
        discovery::{SnmpCredentialMapping, SnmpIpOverride, SnmpQueryCredential},
    },
    tags::entity_tags::EntityTagService,
};
use anyhow::Error;
use async_trait::async_trait;
use chrono::Utc;
use std::sync::{Arc, OnceLock};
use uuid::Uuid;

pub struct SnmpCredentialService {
    storage: Arc<GenericPostgresStorage<SnmpCredential>>,
    event_bus: Arc<EventBus>,
    entity_tag_service: Arc<EntityTagService>,
    network_service: Arc<NetworkService>,
    interface_service: Arc<InterfaceService>,
    organization_service: Arc<OrganizationService>,
    host_service: OnceLock<Arc<HostService>>,
}

impl EventBusService<SnmpCredential> for SnmpCredentialService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, _entity: &SnmpCredential) -> Option<Uuid> {
        None
    }

    fn get_organization_id(&self, entity: &SnmpCredential) -> Option<Uuid> {
        Some(entity.base.organization_id)
    }
}

#[async_trait]
impl CrudService<SnmpCredential> for SnmpCredentialService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<SnmpCredential>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        Some(&self.entity_tag_service)
    }

    async fn create(
        &self,
        entity: SnmpCredential,
        authentication: AuthenticatedEntity,
    ) -> Result<SnmpCredential, Error> {
        let created = self.create_base(entity, authentication.clone()).await?;

        // Emit FirstSnmpCredentialCreated onboarding event if applicable
        let organization_id = created.base.organization_id;
        if let Some(organization) = self
            .organization_service
            .get_by_id(&organization_id)
            .await?
            && organization.not_onboarded(&OnboardingOperation::FirstSnmpCredentialCreated)
        {
            self.event_bus
                .publish_onboarding(OnboardingEvent {
                    id: Uuid::new_v4(),
                    organization_id,
                    operation: OnboardingOperation::FirstSnmpCredentialCreated,
                    timestamp: Utc::now(),
                    metadata: serde_json::json!({}),
                    authentication,
                })
                .await?;
        }

        Ok(created)
    }
}

impl SnmpCredentialService {
    pub fn new(
        storage: Arc<GenericPostgresStorage<SnmpCredential>>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
        network_service: Arc<NetworkService>,
        interface_service: Arc<InterfaceService>,
        organization_service: Arc<OrganizationService>,
    ) -> Self {
        Self {
            storage,
            event_bus,
            entity_tag_service,
            network_service,
            interface_service,
            organization_service,
            host_service: OnceLock::new(),
        }
    }

    // ========================================================================
    // Dependency injection (for breaking circular dependency with HostService)
    // ========================================================================

    /// Set the host service dependency after construction.
    /// This breaks the circular dependency: HostService needs DaemonService,
    /// and DaemonService needs HostService.
    pub fn set_host_service(&self, service: Arc<HostService>) -> Result<(), Arc<HostService>> {
        self.host_service.set(service)
    }

    pub async fn build_credentials_for_discovery(
        &self,
        network_id: Uuid,
    ) -> Result<SnmpCredentialMapping, Error> {
        let host_service = self
            .host_service
            .get()
            .ok_or_else(|| anyhow::anyhow!("HostService not initialized"))?;
        let host_filter = StorableFilter::<Host>::new_from_network_ids(&[network_id]);
        let hosts = host_service.get_all(host_filter).await?;

        let interface_filter = StorableFilter::<Interface>::new_from_network_ids(&[network_id]);
        let interfaces = self.interface_service.get_all(interface_filter).await?;

        let network_credential: Option<SnmpQueryCredential> = if let Some(network) =
            self.network_service.get_by_id(&network_id).await?
            && let Some(cred_id) = network.base.credential_ids.first()
            && let Some(cred) = self.get_by_id(cred_id).await?
        {
            Some(cred.into())
        } else {
            None
        };

        let host_credential_ids: Vec<(Uuid, Uuid)> = hosts
            .iter()
            .filter_map(|h| {
                h.base
                    .credential_assignments
                    .first()
                    .map(|assignment| (h.id, assignment.credential_id))
            })
            .collect();

        let mut overrides: Vec<SnmpIpOverride> = Vec::new();

        for (host_id, snmp_id) in host_credential_ids {
            if let Some(snmp_cred) = self.get_by_id(&snmp_id).await? {
                overrides.extend(
                    interfaces
                        .iter()
                        .filter(|i| i.base.host_id == host_id)
                        .map(|i| SnmpIpOverride {
                            ip: i.base.ip_address,
                            credential: snmp_cred.clone().into(),
                        })
                        .collect::<Vec<SnmpIpOverride>>(),
                );
            }
        }

        Ok(SnmpCredentialMapping {
            default_credential: network_credential,
            ip_overrides: overrides,
        })
    }
}

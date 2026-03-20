use crate::server::{
    auth::middleware::auth::AuthenticatedEntity, credentials::r#impl::{
        base::Credential,
        mapping::{
            CredentialMapping, CredentialQueryPayload, IpOverride, ResolvableSecret,
            SnmpCredentialMapping, SnmpQueryCredential,
        },
        types::{
            CredentialAssignment, CredentialType, CredentialTypeDiscriminants, SecretValue, SnmpVersion
        },
    }, hosts::{r#impl::base::Host, service::HostService}, interfaces::{r#impl::base::Interface, service::InterfaceService}, networks::service::NetworkService, organizations::service::OrganizationService, ports::r#impl::base::PortType, shared::{
        events::{
            bus::EventBus,
            types::{OnboardingEvent, OnboardingOperation},
        },
        services::traits::{CrudService, EventBusService},
        storage::{filter::StorableFilter, generic::GenericPostgresStorage},
    }, tags::entity_tags::EntityTagService
};
use anyhow::Error;
use async_trait::async_trait;
use chrono::Utc;
use secrecy::{ExposeSecret};
use sqlx::PgPool;
use strum::IntoDiscriminant;
use std::sync::{Arc, OnceLock};
use uuid::Uuid;

pub struct CredentialService {
    storage: Arc<GenericPostgresStorage<Credential>>,
    event_bus: Arc<EventBus>,
    entity_tag_service: Arc<EntityTagService>,
    #[allow(dead_code)]
    network_service: Arc<NetworkService>,
    interface_service: Arc<InterfaceService>,
    organization_service: Arc<OrganizationService>,
    host_service: OnceLock<Arc<HostService>>,
    pool: PgPool,
}

impl EventBusService<Credential> for CredentialService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, _entity: &Credential) -> Option<Uuid> {
        None
    }

    fn get_organization_id(&self, entity: &Credential) -> Option<Uuid> {
        Some(entity.base.organization_id)
    }
}

#[async_trait]
impl CrudService<Credential> for CredentialService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Credential>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        Some(&self.entity_tag_service)
    }

    async fn create(
        &self,
        entity: Credential,
        authentication: AuthenticatedEntity,
    ) -> Result<Credential, Error> {
        entity.base.credential_type.validate()?;

        let created = self.create_base(entity, authentication.clone()).await?;

        // Emit onboarding events for credential creation
        let organization_id = created.base.organization_id;
        if let Some(organization) = self
            .organization_service
            .get_by_id(&organization_id)
            .await?
        {
            let now = Utc::now();

            // Generic event for any credential type
            if organization.not_onboarded(&OnboardingOperation::FirstCredentialCreated) {
                self.event_bus
                    .publish_onboarding(OnboardingEvent {
                        id: Uuid::new_v4(),
                        organization_id,
                        operation: OnboardingOperation::FirstCredentialCreated,
                        timestamp: now,
                        metadata: serde_json::json!({}),
                        authentication: authentication.clone(),
                    })
                    .await?;
            }

            // SNMP-specific event (preserves existing Brevo tracking)
            if matches!(created.base.credential_type, CredentialType::SnmpV2c { .. })
                && organization.not_onboarded(&OnboardingOperation::FirstSnmpCredentialCreated)
            {
                self.event_bus
                    .publish_onboarding(OnboardingEvent {
                        id: Uuid::new_v4(),
                        organization_id,
                        operation: OnboardingOperation::FirstSnmpCredentialCreated,
                        timestamp: now,
                        metadata: serde_json::json!({}),
                        authentication,
                    })
                    .await?;
            }
        }

        Ok(created)
    }
}

impl CredentialService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        storage: Arc<GenericPostgresStorage<Credential>>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
        network_service: Arc<NetworkService>,
        interface_service: Arc<InterfaceService>,
        organization_service: Arc<OrganizationService>,
        pool: PgPool,
    ) -> Self {
        Self {
            storage,
            event_bus,
            entity_tag_service,
            network_service,
            interface_service,
            organization_service,
            host_service: OnceLock::new(),
            pool,
        }
    }

    /// Set the host service dependency after construction (breaks circular dep).
    pub fn set_host_service(&self, service: Arc<HostService>) -> Result<(), Arc<HostService>> {
        self.host_service.set(service)
    }

    // ========================================================================
    // Junction table methods
    // ========================================================================

    /// Get credential IDs for a network from the junction table.
    pub async fn get_credential_ids_for_network(
        &self,
        network_id: &Uuid,
    ) -> Result<Vec<Uuid>, Error> {
        let rows = sqlx::query_scalar::<_, Uuid>(
            "SELECT credential_id FROM network_credentials WHERE network_id = $1",
        )
        .bind(network_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Get credential IDs for multiple networks (batch).
    pub async fn get_credential_ids_for_networks(
        &self,
        network_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, Vec<Uuid>>, Error> {
        if network_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let rows: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT network_id, credential_id FROM network_credentials WHERE network_id = ANY($1)",
        )
        .bind(network_ids)
        .fetch_all(&self.pool)
        .await?;

        let mut map: std::collections::HashMap<Uuid, Vec<Uuid>> = std::collections::HashMap::new();
        for (network_id, credential_id) in rows {
            map.entry(network_id).or_default().push(credential_id);
        }
        Ok(map)
    }

    /// Get credential assignments for a host from the junction table.
    pub async fn get_credential_assignments_for_host(
        &self,
        host_id: &Uuid,
    ) -> Result<Vec<CredentialAssignment>, Error> {
        let rows: Vec<(Uuid, Option<Vec<Uuid>>)> = sqlx::query_as(
            "SELECT credential_id, interface_ids FROM host_credentials WHERE host_id = $1",
        )
        .bind(host_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(credential_id, interface_ids)| CredentialAssignment {
                credential_id,
                interface_ids,
            })
            .collect())
    }

    /// Get credential assignments for multiple hosts (batch).
    pub async fn get_credential_assignments_for_hosts(
        &self,
        host_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, Vec<CredentialAssignment>>, Error> {
        if host_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let rows: Vec<(Uuid, Uuid, Option<Vec<Uuid>>)> = sqlx::query_as(
            "SELECT host_id, credential_id, interface_ids FROM host_credentials WHERE host_id = ANY($1)",
        )
        .bind(host_ids)
        .fetch_all(&self.pool)
        .await?;

        let mut map: std::collections::HashMap<Uuid, Vec<CredentialAssignment>> =
            std::collections::HashMap::new();
        for (host_id, credential_id, interface_ids) in rows {
            map.entry(host_id).or_default().push(CredentialAssignment {
                credential_id,
                interface_ids,
            });
        }
        Ok(map)
    }

    /// Replace all credentials for a network (atomic).
    pub async fn set_network_credentials(
        &self,
        network_id: &Uuid,
        credential_ids: &[Uuid],
    ) -> Result<(), Error> {
        let mut tx = sqlx::PgPool::begin(&self.pool).await?;
        sqlx::query("DELETE FROM network_credentials WHERE network_id = $1")
            .bind(network_id)
            .execute(&mut *tx)
            .await?;
        for cred_id in credential_ids {
            sqlx::query(
                "INSERT INTO network_credentials (network_id, credential_id) VALUES ($1, $2)",
            )
            .bind(network_id)
            .bind(cred_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Replace all credential assignments for a host (atomic).
    pub async fn set_host_credentials(
        &self,
        host_id: &Uuid,
        assignments: &[CredentialAssignment],
    ) -> Result<(), Error> {
        let mut tx = sqlx::PgPool::begin(&self.pool).await?;
        sqlx::query("DELETE FROM host_credentials WHERE host_id = $1")
            .bind(host_id)
            .execute(&mut *tx)
            .await?;
        for assignment in assignments {
            sqlx::query(
                "INSERT INTO host_credentials (host_id, credential_id, interface_ids) VALUES ($1, $2, $3)",
            )
            .bind(host_id)
            .bind(assignment.credential_id)
            .bind(&assignment.interface_ids)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    // ========================================================================
    // Discovery credential building
    // ========================================================================

    /// Build SNMP credential mapping for discovery dispatch.
    /// Produces the same SnmpCredentialMapping output as the old SnmpCredentialService.
    pub async fn build_snmp_credentials_for_discovery(
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

        // Get network's SNMP credentials (from junction table)
        let network_cred_ids = self.get_credential_ids_for_network(&network_id).await?;
        tracing::debug!(
            network_id = %network_id,
            credential_count = network_cred_ids.len(),
            "Credential IDs found for network via junction table"
        );
        let mut network_snmp_credential: Option<SnmpQueryCredential> = None;
        for cred_id in &network_cred_ids {
            if let Some(cred) = self.get_by_id(cred_id).await?
                && let CredentialType::SnmpV2c { community } = &cred.base.credential_type
            {
                network_snmp_credential = Some(SnmpQueryCredential {
                    version: SnmpVersion::V2c,
                    community: match community {
                        SecretValue::Inline { value } => ResolvableSecret::Value {
                            value: value.expose_secret().to_string(),
                        },
                        SecretValue::FilePath { path } => {
                            ResolvableSecret::FilePath { path: path.clone() }
                        }
                    },
                });
                break;
            }
        }
        tracing::debug!(
            network_id = %network_id,
            has_default = network_snmp_credential.is_some(),
            "Network default SNMP credential resolution"
        );

        // Get host-level SNMP credential overrides
        let host_ids: Vec<Uuid> = hosts.iter().map(|h| h.id).collect();
        let host_cred_map = self.get_credential_assignments_for_hosts(&host_ids).await?;

        let mut overrides: Vec<IpOverride<SnmpQueryCredential>> = Vec::new();

        for host in &hosts {
            if let Some(assignments) = host_cred_map.get(&host.id) {
                for assignment in assignments {
                    if let Some(cred) = self.get_by_id(&assignment.credential_id).await?
                        && let CredentialType::SnmpV2c { community } = &cred.base.credential_type
                    {
                        let query_cred = SnmpQueryCredential {
                            version: SnmpVersion::V2c,
                            community: match community {
                                SecretValue::Inline { value } => ResolvableSecret::Value {
                                    value: value.expose_secret().to_string(),
                                },
                                SecretValue::FilePath { path } => {
                                    ResolvableSecret::FilePath { path: path.clone() }
                                }
                            },
                        };
                        // If interface_ids is set, only create overrides for those interfaces
                        let relevant_interfaces: Vec<_> = interfaces
                            .iter()
                            .filter(|i| {
                                i.base.host_id == host.id
                                    && match &assignment.interface_ids {
                                        Some(ids) => ids.contains(&i.id),
                                        None => true,
                                    }
                            })
                            .collect();
                        overrides.extend(relevant_interfaces.iter().map(|i| IpOverride {
                            ip: i.base.ip_address,
                            credential: query_cred.clone(),
                            credential_id: cred.id,
                        }));
                        break;
                    }
                }
            }
        }

        tracing::debug!(
            network_id = %network_id,
            ip_overrides = overrides.len(),
            has_default = network_snmp_credential.is_some(),
            "SNMP credential mapping built for discovery"
        );

        Ok(SnmpCredentialMapping {
            default_credential: network_snmp_credential,
            ip_overrides: overrides,
            required_ports: [PortType::Snmp, PortType::SnmpAlt].to_vec()
        })
    }

    /// Build generic credential mappings for unified discovery dispatch.
    /// Returns one `CredentialMapping<CredentialQueryPayload>` per credential type discriminant.
    pub async fn build_credential_mappings_for_discovery(
        &self,
        network_id: Uuid,
    ) -> Result<Vec<CredentialMapping<CredentialQueryPayload>>, Error> {
        let host_service = self
            .host_service
            .get()
            .ok_or_else(|| anyhow::anyhow!("HostService not initialized"))?;

        // Fetch hosts + interfaces on network
        let host_filter = StorableFilter::<Host>::new_from_network_ids(&[network_id]);
        let hosts = host_service.get_all(host_filter).await?;

        let interface_filter = StorableFilter::<Interface>::new_from_network_ids(&[network_id]);
        let interfaces = self.interface_service.get_all(interface_filter).await?;

        // Fetch network-level credentials
        let network_cred_ids = self.get_credential_ids_for_network(&network_id).await?;

        // Group network credentials by discriminant — one mapping per type
        let mut mappings_by_type: std::collections::HashMap<
            CredentialTypeDiscriminants,
            CredentialMapping<CredentialQueryPayload>,
        > = std::collections::HashMap::new();

        for cred_id in &network_cred_ids {
            if let Some(cred) = self.get_by_id(cred_id).await? {
                let cred_type = &cred.base.credential_type;
                let discriminant = cred_type.discriminant();
                let payload = cred_type.to_query_payload();
                let mapping =
                    mappings_by_type
                        .entry(discriminant)
                        .or_insert_with(|| CredentialMapping {
                            default_credential: None,
                            ip_overrides: vec![],
                            required_ports: cred_type.required_ports(),
                        });
                if mapping.default_credential.is_none() {
                    mapping.default_credential = Some(payload);
                }
            }
        }

        // Fetch host-level credential assignments
        let host_ids: Vec<Uuid> = hosts.iter().map(|h| h.id).collect();
        let host_cred_map = self.get_credential_assignments_for_hosts(&host_ids).await?;

        for host in &hosts {
            if let Some(assignments) = host_cred_map.get(&host.id) {
                for assignment in assignments {
                    if let Some(cred) = self.get_by_id(&assignment.credential_id).await? {
                        let cred_type = &cred.base.credential_type;
                        let discriminant = cred_type.discriminant();
                        let payload = cred_type.to_query_payload();
                        let mapping = mappings_by_type.entry(discriminant).or_insert_with(|| {
                            CredentialMapping {
                                default_credential: None,
                                ip_overrides: vec![],
                                required_ports: cred_type.required_ports(),
                            }
                        });

                        // Create IP overrides for relevant interfaces
                        let relevant_interfaces: Vec<_> = interfaces
                            .iter()
                            .filter(|i| {
                                i.base.host_id == host.id
                                    && match &assignment.interface_ids {
                                        Some(ids) => ids.contains(&i.id),
                                        None => true,
                                    }
                            })
                            .collect();

                        mapping
                            .ip_overrides
                            .extend(relevant_interfaces.iter().map(|i| IpOverride {
                                ip: i.base.ip_address,
                                credential: payload.clone(),
                                credential_id: cred.id,
                            }));

                        // Add seed IP overrides (bootstrap IPs for new daemon hosts without interfaces)
                        if let Some(seed_ips) = &cred.base.seed_ips {
                            for ip in seed_ips {
                                mapping.ip_overrides.push(IpOverride {
                                    ip: *ip,
                                    credential: payload.clone(),
                                    credential_id: cred.id,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(mappings_by_type.into_values().collect())
    }

    /// Set seed_ips on a credential.
    pub async fn set_seed_ips(
        &self,
        credential_id: &Uuid,
        seed_ips: Vec<std::net::IpAddr>,
    ) -> Result<(), Error> {
        let networks: Vec<ipnetwork::IpNetwork> = seed_ips
            .iter()
            .map(|ip| ipnetwork::IpNetwork::from(*ip))
            .collect();
        sqlx::query("UPDATE credentials SET seed_ips = $1 WHERE id = $2")
            .bind(&networks)
            .bind(credential_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Clear seed_ips on a credential (set to NULL).
    pub async fn clear_seed_ips(&self, credential_id: &Uuid) -> Result<(), Error> {
        sqlx::query("UPDATE credentials SET seed_ips = NULL WHERE id = $1")
            .bind(credential_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

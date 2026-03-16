use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    credentials::r#impl::{
        base::Credential,
        mapping::{IpOverride, SnmpCredentialMapping, SnmpQueryCredential},
        types::{CredentialAssignment, CredentialType, SecretValue},
    },
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
    tags::entity_tags::EntityTagService,
};
use anyhow::Error;
use async_trait::async_trait;
use chrono::Utc;
use secrecy::ExposeSecret;
use sqlx::PgPool;
use std::sync::{Arc, OnceLock};
use uuid::Uuid;

use crate::bail_validation;

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
        Self::validate_credential_type(&entity.base.credential_type)?;

        let created = self.create_base(entity, authentication.clone()).await?;

        // Emit FirstSnmpCredentialCreated onboarding event if applicable
        let organization_id = created.base.organization_id;
        if matches!(created.base.credential_type, CredentialType::Snmp { .. })
            && let Some(organization) = self
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

    /// Validate PEM format for DockerProxy credential fields.
    pub fn validate_credential_type(credential_type: &CredentialType) -> Result<(), Error> {
        if let CredentialType::DockerProxy {
            ssl_cert,
            ssl_key,
            ssl_chain,
            ..
        } = credential_type
        {
            if let Some(cert) = ssl_cert
                && cert != "********"
            {
                Self::validate_pem_certificate(cert, "SSL Certificate")?;
            }
            if let Some(SecretValue::Inline { value }) = ssl_key {
                let exposed = value.expose_secret();
                if exposed != "********" {
                    Self::validate_pem_private_key(exposed, "SSL Private Key")?;
                }
            }
            // FilePath mode: skip validation (file is on daemon host)
            if let Some(chain) = ssl_chain
                && chain != "********"
            {
                Self::validate_pem_certificate(chain, "SSL CA Chain")?;
            }
        }
        Ok(())
    }

    fn validate_pem_certificate(value: &str, field_name: &str) -> Result<(), Error> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Ok(());
        }
        if !trimmed.starts_with("-----BEGIN CERTIFICATE-----")
            || !trimmed.ends_with("-----END CERTIFICATE-----")
        {
            bail_validation!(
                "{} must be a valid PEM certificate (BEGIN/END CERTIFICATE envelope required)",
                field_name
            );
        }
        Ok(())
    }

    fn validate_pem_private_key(value: &str, field_name: &str) -> Result<(), Error> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Ok(());
        }
        let valid_start = trimmed.starts_with("-----BEGIN PRIVATE KEY-----")
            || trimmed.starts_with("-----BEGIN RSA PRIVATE KEY-----")
            || trimmed.starts_with("-----BEGIN EC PRIVATE KEY-----");
        let valid_end = trimmed.ends_with("-----END PRIVATE KEY-----")
            || trimmed.ends_with("-----END RSA PRIVATE KEY-----")
            || trimmed.ends_with("-----END EC PRIVATE KEY-----");
        if !valid_start || !valid_end {
            bail_validation!(
                "{} must be a valid PEM private key (BEGIN/END PRIVATE KEY envelope required)",
                field_name
            );
        }
        Ok(())
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
        let mut network_snmp_credential: Option<SnmpQueryCredential> = None;
        for cred_id in &network_cred_ids {
            if let Some(cred) = self.get_by_id(cred_id).await?
                && let CredentialType::Snmp { version, community } = &cred.base.credential_type
            {
                network_snmp_credential = Some(SnmpQueryCredential {
                    version: *version,
                    community: redact::Secret::from(community.expose_secret().to_string()),
                });
                break;
            }
        }

        // Get host-level SNMP credential overrides
        let host_ids: Vec<Uuid> = hosts.iter().map(|h| h.id).collect();
        let host_cred_map = self.get_credential_assignments_for_hosts(&host_ids).await?;

        let mut overrides: Vec<IpOverride<SnmpQueryCredential>> = Vec::new();

        for host in &hosts {
            if let Some(assignments) = host_cred_map.get(&host.id) {
                for assignment in assignments {
                    if let Some(cred) = self.get_by_id(&assignment.credential_id).await?
                        && let CredentialType::Snmp { version, community } =
                            &cred.base.credential_type
                    {
                        let query_cred = SnmpQueryCredential {
                            version: *version,
                            community: redact::Secret::from(community.expose_secret().to_string()),
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
                        }));
                        break;
                    }
                }
            }
        }

        Ok(SnmpCredentialMapping {
            default_credential: network_snmp_credential,
            ip_overrides: overrides,
        })
    }
}

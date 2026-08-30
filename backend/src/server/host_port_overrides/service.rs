use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    host_port_overrides::r#impl::base::{HostPortOverride, HostPortOverrideBase},
    shared::{
        events::bus::EventBus,
        services::traits::{ChildCrudService, CrudService, EventBusService},
        storage::{
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            traits::{Storable, Storage},
        },
    },
    tags::entity_tags::EntityTagService,
};

pub struct HostPortOverrideService {
    storage: Arc<GenericPostgresStorage<HostPortOverride>>,
    event_bus: Arc<EventBus>,
}

impl EventBusService<HostPortOverride> for HostPortOverrideService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, entity: &HostPortOverride) -> Option<Uuid> {
        Some(entity.base.network_id)
    }

    fn get_organization_id(&self, _entity: &HostPortOverride) -> Option<Uuid> {
        None
    }
}

impl CrudService<HostPortOverride> for HostPortOverrideService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<HostPortOverride>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        None
    }
}

impl ChildCrudService<HostPortOverride> for HostPortOverrideService {}

impl HostPortOverrideService {
    pub fn new(
        storage: Arc<GenericPostgresStorage<HostPortOverride>>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self { storage, event_bus }
    }

    /// Get all overrides for a specific host.
    pub async fn get_for_host(&self, host_id: &Uuid) -> Result<Vec<HostPortOverride>> {
        self.get_for_parent(host_id).await
    }

    /// Find a single override by its value tuple (host + port number + protocol).
    /// The port row itself is not referenced so overrides survive rescans.
    async fn find_by_tuple(
        &self,
        host_id: &Uuid,
        port_number: u16,
        port_protocol: &str,
    ) -> Result<Option<HostPortOverride>> {
        let filter = StorableFilter::<HostPortOverride>::new_from_uuid_column("host_id", host_id);
        let all = self.storage.get_all(filter).await?;
        Ok(all
            .into_iter()
            .find(|o| o.base.port_number == port_number && o.base.port_protocol == port_protocol))
    }

    /// Upsert a single override for a host+port tuple. Returns the saved entity.
    pub async fn upsert(
        &self,
        base: HostPortOverrideBase,
        authentication: AuthenticatedEntity,
    ) -> Result<HostPortOverride> {
        if let Some(mut existing) = self
            .find_by_tuple(&base.host_id, base.port_number, &base.port_protocol)
            .await?
        {
            existing.base.display_name = base.display_name;
            existing.base.icon_url = base.icon_url;
            existing.base.service_ref_kind = base.service_ref_kind;
            existing.base.service_ref_id = base.service_ref_id;
            self.update(&mut existing, authentication).await
        } else {
            self.create(HostPortOverride::new(base), authentication)
                .await
        }
    }

    /// Remove the override for a host+port tuple (reset to global default).
    /// Returns true if an override was removed.
    pub async fn clear(
        &self,
        host_id: &Uuid,
        port_number: u16,
        port_protocol: &str,
        authentication: AuthenticatedEntity,
    ) -> Result<bool> {
        if let Some(existing) = self
            .find_by_tuple(host_id, port_number, port_protocol)
            .await?
        {
            self.delete(&existing.id, authentication).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

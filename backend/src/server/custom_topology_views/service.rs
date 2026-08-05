use std::sync::Arc;
use uuid::Uuid;

use crate::server::{
    custom_topology_views::r#impl::base::CustomTopologyView,
    shared::{
        events::bus::EventBus,
        services::traits::{CrudService, EventBusService},
        storage::generic::GenericPostgresStorage,
    },
    tags::entity_tags::EntityTagService,
};

pub struct CustomTopologyViewService {
    storage: Arc<GenericPostgresStorage<CustomTopologyView>>,
    event_bus: Arc<EventBus>,
}

impl EventBusService<CustomTopologyView> for CustomTopologyViewService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, entity: &CustomTopologyView) -> Option<Uuid> {
        Some(entity.base.network_id)
    }

    fn get_organization_id(&self, _entity: &CustomTopologyView) -> Option<Uuid> {
        None
    }
}

impl CrudService<CustomTopologyView> for CustomTopologyViewService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<CustomTopologyView>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        None
    }
}

impl CustomTopologyViewService {
    pub fn new(
        storage: Arc<GenericPostgresStorage<CustomTopologyView>>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self { storage, event_bus }
    }
}

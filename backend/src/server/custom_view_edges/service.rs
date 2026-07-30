use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;

use crate::server::{
    custom_view_edges::r#impl::base::CustomViewEdge,
    shared::{
        events::bus::EventBus,
        services::traits::{ChildCrudService, CrudService, EventBusService},
        storage::generic::GenericPostgresStorage,
    },
    tags::entity_tags::EntityTagService,
};

pub struct CustomViewEdgeService {
    storage: Arc<GenericPostgresStorage<CustomViewEdge>>,
    event_bus: Arc<EventBus>,
}

impl EventBusService<CustomViewEdge> for CustomViewEdgeService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, entity: &CustomViewEdge) -> Option<Uuid> {
        Some(entity.base.network_id)
    }

    fn get_organization_id(&self, _entity: &CustomViewEdge) -> Option<Uuid> {
        None
    }
}

impl CrudService<CustomViewEdge> for CustomViewEdgeService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<CustomViewEdge>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        None
    }
}

impl ChildCrudService<CustomViewEdge> for CustomViewEdgeService {}

impl CustomViewEdgeService {
    pub fn new(
        storage: Arc<GenericPostgresStorage<CustomViewEdge>>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self { storage, event_bus }
    }

    pub async fn get_for_view(&self, view_id: &Uuid) -> Result<Vec<CustomViewEdge>> {
        self.get_for_parent(view_id).await
    }
}

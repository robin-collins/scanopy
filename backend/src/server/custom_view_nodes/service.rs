use anyhow::Result;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use uuid::Uuid;

use crate::server::{
    custom_view_nodes::r#impl::base::CustomViewNode,
    shared::{
        events::bus::EventBus,
        services::traits::{ChildCrudService, CrudService, EventBusService},
        storage::generic::GenericPostgresStorage,
    },
    tags::entity_tags::EntityTagService,
};

pub struct CustomViewNodeService {
    storage: Arc<GenericPostgresStorage<CustomViewNode>>,
    event_bus: Arc<EventBus>,
    /// Directory per-node uploaded image `storage_path`s resolve relative to —
    /// same directory host_images/library_objects resolve against.
    data_dir: PathBuf,
}

impl EventBusService<CustomViewNode> for CustomViewNodeService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, entity: &CustomViewNode) -> Option<Uuid> {
        Some(entity.base.network_id)
    }

    fn get_organization_id(&self, _entity: &CustomViewNode) -> Option<Uuid> {
        None
    }
}

impl CrudService<CustomViewNode> for CustomViewNodeService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<CustomViewNode>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        None
    }
}

impl ChildCrudService<CustomViewNode> for CustomViewNodeService {}

impl CustomViewNodeService {
    pub fn new(
        storage: Arc<GenericPostgresStorage<CustomViewNode>>,
        event_bus: Arc<EventBus>,
        data_dir: PathBuf,
    ) -> Self {
        Self {
            storage,
            event_bus,
            data_dir,
        }
    }

    pub async fn get_for_view(&self, view_id: &Uuid) -> Result<Vec<CustomViewNode>> {
        self.get_for_parent(view_id).await
    }

    pub fn absolute_path(&self, storage_path: &str) -> PathBuf {
        self.data_dir.join(storage_path)
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }
}

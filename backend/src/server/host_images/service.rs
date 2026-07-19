use anyhow::Result;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use uuid::Uuid;

use crate::server::{
    host_images::r#impl::base::HostImage,
    shared::{
        events::bus::EventBus,
        services::traits::{ChildCrudService, CrudService, EventBusService},
        storage::generic::GenericPostgresStorage,
    },
    tags::entity_tags::EntityTagService,
};

pub struct HostImageService {
    storage: Arc<GenericPostgresStorage<HostImage>>,
    event_bus: Arc<EventBus>,
    /// Directory `HostImage.base.storage_path` is resolved relative to.
    /// Owned by the service (not the handler) so every consumer resolves
    /// paths the same way.
    data_dir: PathBuf,
}

impl EventBusService<HostImage> for HostImageService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, _entity: &HostImage) -> Option<Uuid> {
        None
    }

    fn get_organization_id(&self, _entity: &HostImage) -> Option<Uuid> {
        None
    }
}

impl CrudService<HostImage> for HostImageService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<HostImage>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        None
    }
}

impl ChildCrudService<HostImage> for HostImageService {}

impl HostImageService {
    pub fn new(
        storage: Arc<GenericPostgresStorage<HostImage>>,
        event_bus: Arc<EventBus>,
        data_dir: PathBuf,
    ) -> Self {
        Self {
            storage,
            event_bus,
            data_dir,
        }
    }

    /// Get all images for a specific host (alias for get_for_parent).
    pub async fn get_for_host(&self, host_id: &Uuid) -> Result<Vec<HostImage>> {
        self.get_for_parent(host_id).await
    }

    /// Resolve an image's `storage_path` against the configured data
    /// directory. `storage_path` is always relative — never trust a stored
    /// path that isn't (this would only happen from DB corruption, not from
    /// any code path that writes it).
    pub fn absolute_path(&self, image: &HostImage) -> PathBuf {
        self.data_dir.join(&image.base.storage_path)
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }
}

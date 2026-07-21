use anyhow::Result;
use async_trait::async_trait;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use uuid::Uuid;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    library_objects::r#impl::base::LibraryObject,
    shared::{
        events::bus::EventBus,
        services::traits::{CrudService, EventBusService},
        storage::generic::GenericPostgresStorage,
    },
    tags::entity_tags::EntityTagService,
};

pub struct LibraryObjectService {
    storage: Arc<GenericPostgresStorage<LibraryObject>>,
    event_bus: Arc<EventBus>,
    /// Directory `LibraryObject.base.storage_path` is resolved relative to —
    /// same directory host_images resolves against, just a different
    /// sub-path (`library-objects/...`).
    data_dir: PathBuf,
}

impl EventBusService<LibraryObject> for LibraryObjectService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, _entity: &LibraryObject) -> Option<Uuid> {
        None
    }

    fn get_organization_id(&self, entity: &LibraryObject) -> Option<Uuid> {
        entity.base.organization_id
    }
}

#[async_trait]
impl CrudService<LibraryObject> for LibraryObjectService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<LibraryObject>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        None
    }

    /// Forces `organization_id` from the authenticated caller rather than
    /// trusting the request body — otherwise a client could POST
    /// `organization_id: null` and have it accepted as a fake "built-in"
    /// (the generic `validate_create_access` check only rejects a *mismatched*
    /// org, and treats `None` as "no restriction").
    async fn create(
        &self,
        mut entity: LibraryObject,
        authentication: AuthenticatedEntity,
    ) -> Result<LibraryObject, anyhow::Error> {
        let organization_id = authentication.organization_id().ok_or_else(|| {
            anyhow::anyhow!("Library objects must be created within an organization")
        })?;
        entity.base.organization_id = Some(organization_id);
        self.create_base(entity, authentication).await
    }
}

impl LibraryObjectService {
    pub fn new(
        storage: Arc<GenericPostgresStorage<LibraryObject>>,
        event_bus: Arc<EventBus>,
        data_dir: PathBuf,
    ) -> Self {
        Self {
            storage,
            event_bus,
            data_dir,
        }
    }

    /// Resolve an object's `storage_path` against the configured data
    /// directory. `storage_path` is always relative — never trust a stored
    /// path that isn't.
    pub fn absolute_path(&self, storage_path: &str) -> PathBuf {
        self.data_dir.join(storage_path)
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }
}

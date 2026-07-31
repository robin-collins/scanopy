use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    categories::r#impl::base::Category,
    shared::{
        events::bus::EventBus,
        services::traits::{CrudService, EventBusService},
        storage::generic::GenericPostgresStorage,
    },
    tags::entity_tags::EntityTagService,
};

pub struct CategoryService {
    storage: Arc<GenericPostgresStorage<Category>>,
    event_bus: Arc<EventBus>,
}

impl EventBusService<Category> for CategoryService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, _entity: &Category) -> Option<Uuid> {
        None
    }

    fn get_organization_id(&self, entity: &Category) -> Option<Uuid> {
        entity.base.organization_id
    }
}

#[async_trait]
impl CrudService<Category> for CategoryService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Category>> {
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
        mut entity: Category,
        authentication: AuthenticatedEntity,
    ) -> Result<Category, anyhow::Error> {
        let organization_id = authentication
            .organization_id()
            .ok_or_else(|| anyhow::anyhow!("Categories must be created within an organization"))?;
        entity.base.organization_id = Some(organization_id);
        self.create_base(entity, authentication).await
    }
}

impl CategoryService {
    pub fn new(storage: Arc<GenericPostgresStorage<Category>>, event_bus: Arc<EventBus>) -> Self {
        Self { storage, event_bus }
    }
}

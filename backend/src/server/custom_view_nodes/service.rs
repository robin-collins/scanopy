use anyhow::Result;
use async_trait::async_trait;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use uuid::Uuid;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    custom_view_nodes::r#impl::{base::CustomViewNode, types::NodeKind},
    shared::{
        entities::ChangeTriggersTopologyStaleness,
        events::{
            bus::EventBus,
            traits::{EntityEventFlags, EntityScope, Event},
            types::EntityOperation,
        },
        services::traits::{ChildCrudService, CrudService, EventBusService},
        storage::{filter::StorableFilter, generic::GenericPostgresStorage, traits::Entity},
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

#[async_trait]
impl CrudService<CustomViewNode> for CustomViewNodeService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<CustomViewNode>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        None
    }

    async fn delete(&self, id: &Uuid, authentication: AuthenticatedEntity) -> Result<()> {
        let mut transaction = self.storage.begin_transaction().await?;
        let deleted = transaction
            .get_by_id_for_update(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("custom_topology_view_nodes with id {id} not found"))?;

        let mut updated_children = Vec::new();
        if deleted.base.kind == NodeKind::Group {
            let children = transaction
                .get_all(StorableFilter::new_from_uuid_column("parent_node_id", id))
                .await?;

            for mut child in children {
                let previous = child.clone();
                child.base.x += deleted.base.x;
                child.base.y += deleted.base.y;
                child.base.parent_node_id = None;
                let updated = transaction.update(&mut child).await?;
                updated_children.push((previous, updated));
            }
        }

        transaction
            .delete_by_filter(StorableFilter::new_from_entity_id(id))
            .await?;
        transaction.commit().await?;

        for (previous, updated) in updated_children {
            if let Some(scope) = EntityScope::from_ids(
                updated.id(),
                updated.clone().into(),
                self.get_network_id(&updated),
                self.get_organization_id(&updated),
            ) {
                self.event_bus
                    .publish(
                        Event::new(scope, EntityOperation::Updated, authentication.clone())
                            .with_flags(EntityEventFlags {
                                trigger_stale: updated.triggers_staleness(Some(previous.clone())),
                                suppress_logs: self.suppress_logs(Some(&previous), Some(&updated)),
                                ..Default::default()
                            }),
                    )
                    .await?;
            }
        }

        if let Some(scope) = EntityScope::from_ids(
            deleted.id(),
            deleted.clone().into(),
            self.get_network_id(&deleted),
            self.get_organization_id(&deleted),
        ) {
            self.event_bus
                .publish(
                    Event::new(scope, EntityOperation::Deleted, authentication).with_flags(
                        EntityEventFlags {
                            trigger_stale: deleted.triggers_staleness(None),
                            suppress_logs: self.suppress_logs(Some(&deleted), None),
                            ..Default::default()
                        },
                    ),
                )
                .await?;
        }

        Ok(())
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

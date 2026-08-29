use anyhow::Result;
use async_trait::async_trait;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use uuid::Uuid;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    custom_view_nodes::{
        r#impl::{base::CustomViewNode, types::NodeKind},
        membership::{validate_membership_graph, validate_node_fields},
    },
    shared::{
        entities::ChangeTriggersTopologyStaleness,
        events::{
            bus::EventBus,
            traits::{EntityEventFlags, EntityScope, Event},
            types::EntityOperation,
        },
        services::traits::{ChildCrudService, CrudService, EventBusService},
        storage::{
            filter::StorableFilter,
            generic::{GenericPostgresStorage, StorageTransaction},
            lock::{DEFAULT_LOCK_TIMEOUT, LockKey},
            traits::{Entity, Storable},
        },
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

    async fn create(
        &self,
        entity: CustomViewNode,
        authentication: AuthenticatedEntity,
    ) -> Result<CustomViewNode> {
        let entity = if entity.id() == Uuid::nil() {
            CustomViewNode::new(entity.base)
        } else {
            entity
        };
        validate_node_fields(&entity)?;

        let mut transaction = self.storage.begin_transaction().await?;
        transaction
            .lock(
                LockKey::CustomTopologyLayout {
                    view_id: entity.base.view_id,
                },
                DEFAULT_LOCK_TIMEOUT,
            )
            .await?;
        validate_candidate_in_transaction(&mut transaction, &entity, None).await?;
        let created = transaction.create(&entity).await?;
        transaction.commit().await?;

        if let Err(error) = self
            .publish_node_event(
                created.clone(),
                None,
                EntityOperation::Created,
                authentication,
            )
            .await
        {
            tracing::error!(node_id = %created.id, %error, "Failed to publish committed node creation event");
        }
        Ok(created)
    }

    async fn update(
        &self,
        entity: &mut CustomViewNode,
        authentication: AuthenticatedEntity,
    ) -> Result<CustomViewNode> {
        let existing = self
            .get_by_id(&entity.id())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Could not find {}", entity))?;
        let view_id = existing.base.view_id;

        let mut transaction = self.storage.begin_transaction().await?;
        transaction
            .lock(
                LockKey::CustomTopologyLayout { view_id },
                DEFAULT_LOCK_TIMEOUT,
            )
            .await?;
        let current = transaction
            .get_by_id_for_update(&existing.id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Could not find {}", entity))?;

        // A node cannot be moved between views or networks through its update
        // endpoint. Membership changes are expressed only by parent + x/y.
        entity.id = current.id;
        entity.created_at = current.created_at;
        entity.base.view_id = current.base.view_id;
        entity.base.network_id = current.base.network_id;
        validate_node_fields(entity)?;
        validate_candidate_in_transaction(&mut transaction, entity, Some(entity.id)).await?;

        let updated = transaction.update(entity).await?;
        transaction.commit().await?;
        if let Err(error) = self
            .publish_node_event(
                updated.clone(),
                Some(current),
                EntityOperation::Updated,
                authentication,
            )
            .await
        {
            tracing::error!(node_id = %updated.id, %error, "Failed to publish committed node update event");
        }
        Ok(updated)
    }

    async fn delete(&self, id: &Uuid, authentication: AuthenticatedEntity) -> Result<()> {
        let existing = self
            .get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("custom_topology_view_nodes with id {id} not found"))?;
        let mut transaction = self.storage.begin_transaction().await?;
        transaction
            .lock(
                LockKey::CustomTopologyLayout {
                    view_id: existing.base.view_id,
                },
                DEFAULT_LOCK_TIMEOUT,
            )
            .await?;
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
            ) && let Err(error) =
                self.event_bus
                    .publish(
                        Event::new(scope, EntityOperation::Updated, authentication.clone())
                            .with_flags(EntityEventFlags {
                                trigger_stale: updated.triggers_staleness(Some(previous.clone())),
                                suppress_logs: self.suppress_logs(Some(&previous), Some(&updated)),
                                ..Default::default()
                            }),
                    )
                    .await
            {
                tracing::error!(node_id = %updated.id, %error, "Failed to publish committed child ungroup event");
            }
        }

        if let Some(scope) = EntityScope::from_ids(
            deleted.id(),
            deleted.clone().into(),
            self.get_network_id(&deleted),
            self.get_organization_id(&deleted),
        ) && let Err(error) = self
            .event_bus
            .publish(
                Event::new(scope, EntityOperation::Deleted, authentication).with_flags(
                    EntityEventFlags {
                        trigger_stale: deleted.triggers_staleness(None),
                        suppress_logs: self.suppress_logs(Some(&deleted), None),
                        ..Default::default()
                    },
                ),
            )
            .await
        {
            tracing::error!(node_id = %deleted.id, %error, "Failed to publish committed group deletion event");
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

    async fn publish_node_event(
        &self,
        node: CustomViewNode,
        previous: Option<CustomViewNode>,
        operation: EntityOperation,
        authentication: AuthenticatedEntity,
    ) -> Result<()> {
        if let Some(scope) = EntityScope::from_ids(
            node.id(),
            node.clone().into(),
            self.get_network_id(&node),
            self.get_organization_id(&node),
        ) {
            self.event_bus
                .publish(Event::new(scope, operation, authentication).with_flags(
                    EntityEventFlags {
                        trigger_stale: node.triggers_staleness(previous.clone()),
                        suppress_logs: self.suppress_logs(previous.as_ref(), Some(&node)),
                        ..Default::default()
                    },
                ))
                .await?;
        }
        Ok(())
    }
}

async fn validate_candidate_in_transaction(
    transaction: &mut StorageTransaction<'_, CustomViewNode>,
    candidate: &CustomViewNode,
    replaced_id: Option<Uuid>,
) -> Result<()> {
    let mut nodes = transaction
        .get_all(StorableFilter::new_from_uuid_column(
            "view_id",
            &candidate.base.view_id,
        ))
        .await?;
    if let Some(replaced_id) = replaced_id {
        nodes.retain(|node| node.id != replaced_id);
    }
    nodes.push(candidate.clone());
    validate_membership_graph(&nodes, candidate.base.view_id)?;
    Ok(())
}

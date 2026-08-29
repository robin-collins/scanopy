use std::collections::{HashMap, HashSet};

use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;
use validator::Validate;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    custom_view_edges::{r#impl::base::CustomViewEdge, service::CustomViewEdgeService},
    custom_view_nodes::{
        r#impl::base::CustomViewNode,
        membership::{validate_membership_graph, validate_node_fields},
        service::CustomViewNodeService,
    },
    shared::{
        entities::ChangeTriggersTopologyStaleness,
        events::{
            traits::{EntityEventFlags, EntityScope, Event},
            types::EntityOperation,
        },
        services::traits::{CrudService, EventBusService},
        storage::{
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            lock::{DEFAULT_LOCK_TIMEOUT, LockKey, xact_lock},
            traits::{Entity, Storable},
        },
        types::api::ValidationError,
    },
};

pub struct SavedLayout {
    pub nodes: Vec<CustomViewNode>,
    pub edges: Vec<CustomViewEdge>,
}

/// Validate and persist one complete layout change in a single transaction.
/// Event publication is deliberately post-commit and best-effort: a subscriber
/// failure cannot turn a committed layout into an apparent failed save.
pub async fn save_layout(
    node_service: &CustomViewNodeService,
    edge_service: &CustomViewEdgeService,
    view_id: Uuid,
    network_id: Uuid,
    nodes: Vec<CustomViewNode>,
    edges: Vec<CustomViewEdge>,
    authentication: AuthenticatedEntity,
) -> Result<SavedLayout> {
    let mut transaction = node_service.storage().pool().begin().await?;
    xact_lock(
        &mut transaction,
        LockKey::CustomTopologyLayout { view_id },
        DEFAULT_LOCK_TIMEOUT,
    )
    .await?;

    let existing_nodes = GenericPostgresStorage::<CustomViewNode>::get_all_in_tx(
        StorableFilter::new_from_uuid_column("view_id", &view_id),
        &mut transaction,
    )
    .await?;
    let existing_edges = GenericPostgresStorage::<CustomViewEdge>::get_all_in_tx(
        StorableFilter::new_from_uuid_column("view_id", &view_id),
        &mut transaction,
    )
    .await?;

    let mut final_nodes: HashMap<Uuid, CustomViewNode> = existing_nodes
        .iter()
        .cloned()
        .map(|node| (node.id, node))
        .collect();
    let existing_nodes_by_id = final_nodes.clone();
    let mut seen_node_ids = HashSet::new();
    let mut created_nodes = Vec::new();
    let mut updated_nodes = Vec::new();

    for mut node in nodes {
        node.base.view_id = view_id;
        node.base.network_id = network_id;
        if node.id() == Uuid::nil() {
            node = CustomViewNode::new(node.base);
            validate_node_fields(&node)?;
            final_nodes.insert(node.id, node.clone());
            created_nodes.push(node);
            continue;
        }
        if !seen_node_ids.insert(node.id) {
            return Err(
                ValidationError::new("A layout change cannot contain duplicate node IDs").into(),
            );
        }
        let previous = existing_nodes_by_id.get(&node.id).ok_or_else(|| {
            ValidationError::new("Updated nodes must already belong to the target custom view")
        })?;
        node.created_at = previous.created_at;
        node.updated_at = Utc::now();
        node.base.storage_path = previous.base.storage_path.clone();
        node.base.content_type = previous.base.content_type.clone();
        node.base.size_bytes = previous.base.size_bytes;
        validate_node_fields(&node)?;
        final_nodes.insert(node.id, node.clone());
        updated_nodes.push((previous.clone(), node));
    }

    let final_node_list: Vec<_> = final_nodes.values().cloned().collect();
    validate_membership_graph(&final_node_list, view_id)?;

    let existing_edges_by_id: HashMap<_, _> = existing_edges
        .into_iter()
        .map(|edge| (edge.id, edge))
        .collect();
    let mut seen_edge_ids = HashSet::new();
    let mut created_edges = Vec::new();
    let mut updated_edges = Vec::new();
    for mut edge in edges {
        edge.base.view_id = view_id;
        edge.base.network_id = network_id;
        validate_edge_endpoints(&edge, &final_nodes)?;
        if edge.id() == Uuid::nil() {
            edge = CustomViewEdge::new(edge.base);
            validate_edge_fields(&edge)?;
            created_edges.push(edge);
            continue;
        }
        if !seen_edge_ids.insert(edge.id) {
            return Err(
                ValidationError::new("A layout change cannot contain duplicate edge IDs").into(),
            );
        }
        let previous = existing_edges_by_id.get(&edge.id).ok_or_else(|| {
            ValidationError::new("Updated edges must already belong to the target custom view")
        })?;
        edge.created_at = previous.created_at;
        edge.updated_at = Utc::now();
        validate_edge_fields(&edge)?;
        updated_edges.push((previous.clone(), edge));
    }

    GenericPostgresStorage::<CustomViewNode>::create_many_in_tx(&created_nodes, &mut transaction)
        .await?;
    GenericPostgresStorage::<CustomViewNode>::update_many_in_tx(
        &updated_nodes
            .iter()
            .map(|(_, node)| node.clone())
            .collect::<Vec<_>>(),
        &mut transaction,
    )
    .await?;
    GenericPostgresStorage::<CustomViewEdge>::create_many_in_tx(&created_edges, &mut transaction)
        .await?;
    GenericPostgresStorage::<CustomViewEdge>::update_many_in_tx(
        &updated_edges
            .iter()
            .map(|(_, edge)| edge.clone())
            .collect::<Vec<_>>(),
        &mut transaction,
    )
    .await?;
    transaction.commit().await?;

    publish_node_events(
        node_service,
        &created_nodes,
        &updated_nodes,
        authentication.clone(),
    )
    .await;
    publish_edge_events(edge_service, &created_edges, &updated_edges, authentication).await;

    Ok(SavedLayout {
        nodes: created_nodes
            .into_iter()
            .chain(updated_nodes.into_iter().map(|(_, node)| node))
            .collect(),
        edges: created_edges
            .into_iter()
            .chain(updated_edges.into_iter().map(|(_, edge)| edge))
            .collect(),
    })
}

fn validate_edge_fields(edge: &CustomViewEdge) -> Result<()> {
    edge.validate()
        .map_err(|error| ValidationError::new(format!("Invalid custom view edge: {error}")).into())
}

fn validate_edge_endpoints(
    edge: &CustomViewEdge,
    nodes: &HashMap<Uuid, CustomViewNode>,
) -> Result<()> {
    if !nodes.contains_key(&edge.base.source_node_id)
        || !nodes.contains_key(&edge.base.target_node_id)
    {
        return Err(ValidationError::new(
            "Edge endpoints must both belong to the target custom view",
        )
        .into());
    }
    Ok(())
}

async fn publish_node_events(
    service: &CustomViewNodeService,
    created: &[CustomViewNode],
    updated: &[(CustomViewNode, CustomViewNode)],
    authentication: AuthenticatedEntity,
) {
    for (previous, node, operation) in created
        .iter()
        .map(|node| (None, node, EntityOperation::Created))
        .chain(
            updated
                .iter()
                .map(|(previous, node)| (Some(previous), node, EntityOperation::Updated)),
        )
    {
        let Some(scope) = EntityScope::from_ids(
            node.id(),
            node.clone().into(),
            service.get_network_id(node),
            service.get_organization_id(node),
        ) else {
            continue;
        };
        let event =
            Event::new(scope, operation, authentication.clone()).with_flags(EntityEventFlags {
                trigger_stale: node.triggers_staleness(previous.cloned()),
                suppress_logs: service.suppress_logs(previous, Some(node)),
                ..Default::default()
            });
        if let Err(error) = service.event_bus().publish(event).await {
            tracing::error!(node_id = %node.id, %error, "Failed to publish committed layout node event");
        }
    }
}

async fn publish_edge_events(
    service: &CustomViewEdgeService,
    created: &[CustomViewEdge],
    updated: &[(CustomViewEdge, CustomViewEdge)],
    authentication: AuthenticatedEntity,
) {
    for (previous, edge, operation) in created
        .iter()
        .map(|edge| (None, edge, EntityOperation::Created))
        .chain(
            updated
                .iter()
                .map(|(previous, edge)| (Some(previous), edge, EntityOperation::Updated)),
        )
    {
        let Some(scope) = EntityScope::from_ids(
            edge.id(),
            edge.clone().into(),
            service.get_network_id(edge),
            service.get_organization_id(edge),
        ) else {
            continue;
        };
        let event =
            Event::new(scope, operation, authentication.clone()).with_flags(EntityEventFlags {
                trigger_stale: edge.triggers_staleness(previous.cloned()),
                suppress_logs: service.suppress_logs(previous, Some(edge)),
                ..Default::default()
            });
        if let Err(error) = service.event_bus().publish(event).await {
            tracing::error!(edge_id = %edge.id, %error, "Failed to publish committed layout edge event");
        }
    }
}

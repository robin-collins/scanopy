use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    custom_topology_views::{
        r#impl::base::{CustomTopologyView, CustomTopologyViewBase},
        layout::save_layout,
    },
    custom_view_nodes::r#impl::{
        base::{CustomViewNode, CustomViewNodeBase},
        types::NodeKind,
    },
    shared::{
        services::traits::CrudService,
        storage::traits::{Storable, Storage},
    },
};

use super::{network, organization, test_services, user};

#[tokio::test]
async fn deleting_group_preserves_child_absolute_position() {
    let (storage, services, _container) = test_services().await;

    let organization = organization();
    storage.organizations.create(&organization).await.unwrap();
    storage.users.create(&user(&organization.id)).await.unwrap();
    let network = network(&organization.id);
    storage.networks.create(&network).await.unwrap();

    let view = services
        .custom_topology_view_service
        .create(
            CustomTopologyView::new(CustomTopologyViewBase {
                network_id: network.id,
                name: "Group deletion".to_string(),
                ..Default::default()
            }),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    let group = services
        .custom_view_node_service
        .create(
            CustomViewNode::new(CustomViewNodeBase {
                view_id: view.id,
                network_id: network.id,
                kind: NodeKind::Group,
                x: 120,
                y: 80,
                ..Default::default()
            }),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();
    let child = services
        .custom_view_node_service
        .create(
            CustomViewNode::new(CustomViewNodeBase {
                view_id: view.id,
                network_id: network.id,
                kind: NodeKind::Text,
                parent_node_id: Some(group.id),
                x: 35,
                y: 25,
                ..Default::default()
            }),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    services
        .custom_view_node_service
        .delete(&group.id, AuthenticatedEntity::System)
        .await
        .unwrap();

    assert!(
        services
            .custom_view_node_service
            .get_by_id(&group.id)
            .await
            .unwrap()
            .is_none()
    );
    let preserved_child = services
        .custom_view_node_service
        .get_by_id(&child.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(preserved_child.base.parent_node_id, None);
    assert_eq!(preserved_child.base.x, 155);
    assert_eq!(preserved_child.base.y, 105);
}

#[tokio::test]
async fn rejects_invalid_group_membership_through_the_service() {
    let (storage, services, _container) = test_services().await;

    let organization = organization();
    storage.organizations.create(&organization).await.unwrap();
    storage.users.create(&user(&organization.id)).await.unwrap();
    let network = network(&organization.id);
    storage.networks.create(&network).await.unwrap();
    let first_view = services
        .custom_topology_view_service
        .create(
            CustomTopologyView::new(CustomTopologyViewBase {
                network_id: network.id,
                name: "Membership validation one".to_string(),
                ..Default::default()
            }),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();
    let second_view = services
        .custom_topology_view_service
        .create(
            CustomTopologyView::new(CustomTopologyViewBase {
                network_id: network.id,
                name: "Membership validation two".to_string(),
                ..Default::default()
            }),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();
    let group = services
        .custom_view_node_service
        .create(
            CustomViewNode::new(CustomViewNodeBase {
                view_id: first_view.id,
                network_id: network.id,
                kind: NodeKind::Group,
                ..Default::default()
            }),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();
    let text = services
        .custom_view_node_service
        .create(
            CustomViewNode::new(CustomViewNodeBase {
                view_id: first_view.id,
                network_id: network.id,
                kind: NodeKind::Text,
                parent_node_id: Some(group.id),
                ..Default::default()
            }),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    let nested_group = CustomViewNode::new(CustomViewNodeBase {
        view_id: first_view.id,
        network_id: network.id,
        kind: NodeKind::Group,
        parent_node_id: Some(group.id),
        ..Default::default()
    });
    let nested_error = services
        .custom_view_node_service
        .create(nested_group, AuthenticatedEntity::System)
        .await
        .unwrap_err();
    assert!(
        nested_error
            .to_string()
            .contains("Group frames cannot be children")
    );

    let invalid_kind_child = CustomViewNode::new(CustomViewNodeBase {
        view_id: first_view.id,
        network_id: network.id,
        kind: NodeKind::Entity,
        parent_node_id: Some(text.id),
        ..Default::default()
    });
    let kind_error = services
        .custom_view_node_service
        .create(invalid_kind_child, AuthenticatedEntity::System)
        .await
        .unwrap_err();
    assert!(kind_error.to_string().contains("Only group frames"));

    let cross_view_child = CustomViewNode::new(CustomViewNodeBase {
        view_id: second_view.id,
        network_id: network.id,
        kind: NodeKind::Entity,
        parent_node_id: Some(group.id),
        ..Default::default()
    });
    let cross_view_error = services
        .custom_view_node_service
        .create(cross_view_child, AuthenticatedEntity::System)
        .await
        .unwrap_err();
    assert!(
        cross_view_error
            .to_string()
            .contains("exist and belong to the same")
    );

    let mut demoted_group = group.clone();
    demoted_group.base.kind = NodeKind::Text;
    let demotion_error = services
        .custom_view_node_service
        .update(&mut demoted_group, AuthenticatedEntity::System)
        .await
        .unwrap_err();
    assert!(demotion_error.to_string().contains("Only group frames"));
    let persisted_group = services
        .custom_view_node_service
        .get_by_id(&group.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(persisted_group.base.kind, NodeKind::Group);
}

#[tokio::test]
async fn failed_layout_save_rolls_back_every_node_change() {
    let (storage, services, _container) = test_services().await;

    let organization = organization();
    storage.organizations.create(&organization).await.unwrap();
    storage.users.create(&user(&organization.id)).await.unwrap();
    let network = network(&organization.id);
    storage.networks.create(&network).await.unwrap();
    let view = services
        .custom_topology_view_service
        .create(
            CustomTopologyView::new(CustomTopologyViewBase {
                network_id: network.id,
                name: "Atomic layout".to_string(),
                ..Default::default()
            }),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();
    let first = services
        .custom_view_node_service
        .create(
            CustomViewNode::new(CustomViewNodeBase {
                view_id: view.id,
                network_id: network.id,
                kind: NodeKind::Text,
                x: 10,
                y: 20,
                ..Default::default()
            }),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();
    let second = services
        .custom_view_node_service
        .create(
            CustomViewNode::new(CustomViewNodeBase {
                view_id: view.id,
                network_id: network.id,
                kind: NodeKind::Text,
                x: 30,
                y: 40,
                ..Default::default()
            }),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    let mut valid_first_update = first.clone();
    valid_first_update.base.x = 500;
    valid_first_update.base.y = 600;
    let mut database_rejected_second_update = second.clone();
    database_rejected_second_update.base.x = 2_000_000;

    let result = save_layout(
        &services.custom_view_node_service,
        &services.custom_view_edge_service,
        view.id,
        network.id,
        vec![valid_first_update, database_rejected_second_update],
        vec![],
        AuthenticatedEntity::System,
    )
    .await;
    assert!(result.is_err());

    let persisted_first = services
        .custom_view_node_service
        .get_by_id(&first.id)
        .await
        .unwrap()
        .unwrap();
    let persisted_second = services
        .custom_view_node_service
        .get_by_id(&second.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!((persisted_first.base.x, persisted_first.base.y), (10, 20));
    assert_eq!((persisted_second.base.x, persisted_second.base.y), (30, 40));
}

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    custom_topology_views::r#impl::base::{CustomTopologyView, CustomTopologyViewBase},
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

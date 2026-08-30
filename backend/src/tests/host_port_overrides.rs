//! Database regressions for per-host port overrides.

use std::sync::Arc;

use sqlx::error::DatabaseError;
use uuid::Uuid;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    custom_service_definitions::handlers::guard_custom_service_deletion,
    host_port_overrides::{
        r#impl::base::{HostPortOverrideBase, ServiceRefKind},
        service::HostPortOverrideService,
    },
    ports::r#impl::base::{Port, PortBase, PortType},
    shared::{events::bus::EventBus, storage::traits::Storage},
};

use super::{host, network, organization, test_storage};

async fn fixture() -> (
    crate::server::shared::storage::factory::StorageFactory,
    testcontainers::ContainerAsync<testcontainers::GenericImage>,
    crate::server::hosts::r#impl::base::Host,
) {
    let (storage, container) = test_storage().await;
    let organization = organization();
    storage.organizations.create(&organization).await.unwrap();
    let network = network(&organization.id);
    storage.networks.create(&network).await.unwrap();
    let host = host(&network.id);
    let host = storage.hosts.create(&host).await.unwrap();
    (storage, container, host)
}

fn service(
    storage: &crate::server::shared::storage::factory::StorageFactory,
) -> HostPortOverrideService {
    HostPortOverrideService::new(
        storage.host_port_overrides.clone(),
        Arc::new(EventBus::new()),
    )
}

#[tokio::test]
async fn value_key_survives_port_row_recreation_and_fields_can_be_reset() {
    let (storage, _container, host) = fixture().await;
    let port_type = PortType::new_tcp(17443);
    let old_port = storage
        .ports
        .create(&Port::new(PortBase::new(
            host.id,
            host.base.network_id,
            port_type,
        )))
        .await
        .unwrap();

    let override_service = service(&storage);
    let created = override_service
        .upsert(
            HostPortOverrideBase {
                host_id: host.id,
                network_id: host.base.network_id,
                port_number: 17443,
                port_protocol: "Tcp".to_string(),
                display_name: Some("Private dashboard".to_string()),
                icon_url: Some("https://example.test/dashboard.svg".to_string()),
                service_ref_kind: Some(ServiceRefKind::BuiltIn),
                service_ref_id: Some("HTTP Server".to_string()),
            },
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    storage.ports.delete(&old_port.id).await.unwrap();
    let replacement_port = storage
        .ports
        .create(&Port::new(PortBase::new(
            host.id,
            host.base.network_id,
            port_type,
        )))
        .await
        .unwrap();
    assert_ne!(old_port.id, replacement_port.id);

    let after_rescan = override_service.get_for_host(&host.id).await.unwrap();
    assert_eq!(after_rescan.len(), 1);
    assert_eq!(after_rescan[0].id, created.id);
    assert_eq!(after_rescan[0].base.port_number, 17443);
    assert_eq!(after_rescan[0].base.port_protocol, "Tcp");
    assert_eq!(
        after_rescan[0].base.display_name.as_deref(),
        Some("Private dashboard")
    );

    let reset = override_service
        .upsert(
            HostPortOverrideBase {
                host_id: host.id,
                network_id: host.base.network_id,
                port_number: 17443,
                port_protocol: "Tcp".to_string(),
                display_name: None,
                icon_url: None,
                service_ref_kind: None,
                service_ref_id: None,
            },
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();
    assert_eq!(
        reset.id, created.id,
        "upsert must retain the value-keyed row"
    );
    assert_eq!(reset.base.display_name, None);
    assert_eq!(reset.base.icon_url, None);
    assert_eq!(reset.base.service_ref_kind, None);
    assert_eq!(reset.base.service_ref_id, None);

    assert!(
        override_service
            .clear(&host.id, 17443, "Tcp", AuthenticatedEntity::System)
            .await
            .unwrap()
    );
    assert!(
        override_service
            .get_for_host(&host.id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn database_rejects_half_set_service_reference_pairs() {
    let (storage, _container, host) = fixture().await;

    for (kind, id) in [(Some("BuiltIn"), None), (None, Some("HTTP Server"))] {
        let error = sqlx::query(
            "INSERT INTO host_port_overrides \
             (id, host_id, network_id, port_number, port_protocol, service_ref_kind, service_ref_id) \
             VALUES ($1, $2, $3, $4, 'Tcp', $5, $6)",
        )
        .bind(Uuid::new_v4())
        .bind(host.id)
        .bind(host.base.network_id)
        .bind(if kind.is_some() { 17444_i64 } else { 17445_i64 })
        .bind(kind)
        .bind(id)
        .execute(&storage.pool)
        .await
        .expect_err("the database pairing CHECK must reject a half-set service reference");

        let constraint = error
            .as_database_error()
            .and_then(DatabaseError::constraint);
        assert_eq!(
            constraint,
            Some("host_port_overrides_service_ref_pairing_check")
        );
    }
}

#[tokio::test]
async fn referenced_custom_service_delete_guard_names_the_host() {
    let (storage, _container, host) = fixture().await;
    let custom_service_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO host_port_overrides \
         (id, host_id, network_id, port_number, port_protocol, service_ref_kind, service_ref_id) \
         VALUES ($1, $2, $3, 17446, 'Tcp', 'Custom', $4)",
    )
    .bind(Uuid::new_v4())
    .bind(host.id)
    .bind(host.base.network_id)
    .bind(custom_service_id.to_string())
    .execute(&storage.pool)
    .await
    .unwrap();

    let organization_id: Uuid =
        sqlx::query_scalar("SELECT organization_id FROM networks WHERE id = $1")
            .bind(host.base.network_id)
            .fetch_one(&storage.pool)
            .await
            .unwrap();
    let error = guard_custom_service_deletion(&storage.pool, &organization_id, &custom_service_id)
        .await
        .expect_err("a referenced custom service must be blocked from deletion");

    assert_eq!(error.status, axum::http::StatusCode::CONFLICT);
    assert!(
        error.message.contains("Test Host"),
        "the conflict must name the referencing host: {}",
        error.message
    );
    assert!(
        guard_custom_service_deletion(&storage.pool, &Uuid::new_v4(), &custom_service_id)
            .await
            .is_ok(),
        "a reference in another organization must not block deletion"
    );
}

#[tokio::test]
async fn dangling_service_reference_round_trips_as_its_raw_id() {
    let (storage, _container, host) = fixture().await;
    let dangling_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO host_port_overrides \
         (id, host_id, network_id, port_number, port_protocol, service_ref_kind, service_ref_id) \
         VALUES ($1, $2, $3, 17447, 'Tcp', 'Custom', $4)",
    )
    .bind(Uuid::new_v4())
    .bind(host.id)
    .bind(host.base.network_id)
    .bind(&dangling_id)
    .execute(&storage.pool)
    .await
    .unwrap();

    let overrides = service(&storage).get_for_host(&host.id).await.unwrap();
    assert_eq!(overrides.len(), 1);
    assert_eq!(
        overrides[0].base.service_ref_kind,
        Some(ServiceRefKind::Custom)
    );
    assert_eq!(
        overrides[0].base.service_ref_id.as_deref(),
        Some(dangling_id.as_str())
    );
}

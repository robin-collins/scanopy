//! Database coverage for the custom layer of the Known Ports catalogue.

use crate::server::{
    known_ports::{
        service::{KnownPortService, KnownPortServiceError},
        types::{CatalogueSource, KnownPortInput},
    },
    ports::r#impl::base::TransportProtocol,
    shared::storage::traits::Storage,
};

use super::{organization, test_storage};

fn input(name: &str, port_number: u16) -> KnownPortInput {
    KnownPortInput {
        name: name.to_string(),
        description: Some(format!("Description for {name}")),
        port_number,
        transport_protocol: TransportProtocol::Tcp,
    }
}

#[tokio::test]
async fn custom_known_ports_are_org_scoped_and_complete_the_crud_lifecycle() {
    let (storage, _container) = test_storage().await;
    let first_org = organization();
    let second_org = organization();
    storage.organizations.create(&first_org).await.unwrap();
    storage.organizations.create(&second_org).await.unwrap();

    let service = KnownPortService::new(storage.pool.clone());
    let built_in_count = service
        .list(first_org.id)
        .await
        .unwrap()
        .into_iter()
        .filter(|port| port.source == CatalogueSource::BuiltIn)
        .count();

    let created = service
        .create(first_org.id, input("Internal Dashboard", 17443))
        .await
        .unwrap();
    assert_eq!(created.source, CatalogueSource::Custom);
    assert_eq!(created.organization_id, Some(first_org.id));

    let first_org_ports = service.list(first_org.id).await.unwrap();
    assert_eq!(first_org_ports.len(), built_in_count + 1);
    assert!(first_org_ports.iter().any(|port| port.id == created.id));
    assert_eq!(
        service.list(second_org.id).await.unwrap().len(),
        built_in_count
    );

    let duplicate = service
        .create(first_org.id, input("Duplicate Endpoint", 17443))
        .await
        .expect_err("the database must enforce one custom definition per endpoint");
    assert!(matches!(duplicate, KnownPortServiceError::Conflict));

    let id = created.id.parse().unwrap();
    let updated = service
        .update(first_org.id, id, input("Renamed Dashboard", 17444))
        .await
        .unwrap();
    assert_eq!(updated.name, "Renamed Dashboard");
    assert_eq!(updated.port_number, 17444);

    let cross_org_update = service
        .update(second_org.id, id, input("Cross-org edit", 17445))
        .await
        .expect_err("another organization must not update this definition");
    assert!(matches!(cross_org_update, KnownPortServiceError::NotFound));

    service.delete(first_org.id, id).await.unwrap();
    assert_eq!(
        service.list(first_org.id).await.unwrap().len(),
        built_in_count
    );
    assert!(matches!(
        service.delete(first_org.id, id).await,
        Err(KnownPortServiceError::NotFound)
    ));
}

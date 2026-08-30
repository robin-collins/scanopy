use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use strum::IntoEnumIterator;
use uuid::Uuid;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    custom_service_definitions::r#impl::base::CustomServiceDefinition,
    services::definitions::ServiceDefinitionRegistry,
    services::r#impl::categories::ServiceCategory,
    shared::{
        events::bus::EventBus,
        services::traits::{CrudService, EventBusService},
        storage::generic::GenericPostgresStorage,
        types::api::ValidationError,
        types::metadata::HasId,
    },
    tags::entity_tags::EntityTagService,
};

pub struct CustomServiceDefinitionService {
    storage: Arc<GenericPostgresStorage<CustomServiceDefinition>>,
    event_bus: Arc<EventBus>,
}

impl EventBusService<CustomServiceDefinition> for CustomServiceDefinitionService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, _entity: &CustomServiceDefinition) -> Option<Uuid> {
        None
    }

    fn get_organization_id(&self, _entity: &CustomServiceDefinition) -> Option<Uuid> {
        None
    }
}

#[async_trait]
impl CrudService<CustomServiceDefinition> for CustomServiceDefinitionService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<CustomServiceDefinition>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        None
    }

    async fn create(
        &self,
        mut entity: CustomServiceDefinition,
        authentication: AuthenticatedEntity,
    ) -> Result<CustomServiceDefinition, anyhow::Error> {
        Self::validate_custom_definition(&mut entity)?;
        self.create_base(entity, authentication).await
    }
}

impl CustomServiceDefinitionService {
    pub fn new(
        storage: Arc<GenericPostgresStorage<CustomServiceDefinition>>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self { storage, event_bus }
    }

    /// Validate a readied definition before it is persisted (create or update).
    ///
    /// - trims the `name` (the database enforces uniqueness on `lower(name)`,
    ///   so a trailing/leading space would let hidden duplicates in);
    /// - rejects a name that collides — case-insensitively — with a *built-in*
    ///   definition id. Built-ins are compile-time and read-only, so this is
    ///   the rule that keeps the built-in namespace protected; it is enforced
    ///   in the backend, not just the UI, so a custom row can never shadow or
    ///   overwrite a built-in;
    /// - rejects a `category` that is not a real `ServiceCategory` id, so
    ///   garbage TEXT cannot be persisted and break every consumer of the
    ///   merged catalogue.
    pub fn validate_custom_definition(
        entity: &mut CustomServiceDefinition,
    ) -> Result<(), anyhow::Error> {
        let name = entity.base.name.trim().to_string();
        entity.base.name = name.clone();

        if entity.base.name.is_empty() {
            return Err(ValidationError::new("Name must be between 1 and 39 characters").into());
        }

        if !ServiceCategory::iter().any(|category| category.id() == entity.base.category) {
            return Err(ValidationError::new(format!(
                "Invalid service category '{}'",
                entity.base.category
            ))
            .into());
        }

        let collides = ServiceDefinitionRegistry::all_service_definitions()
            .iter()
            .any(|definition| definition.id().eq_ignore_ascii_case(&entity.base.name));
        if collides {
            return Err(ValidationError::new(
                "Custom service name conflicts with an existing built-in service. \
                 Built-in definitions are read-only and cannot be overridden.",
            )
            .into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::custom_service_definitions::r#impl::base::CustomServiceDefinition;

    fn definition_with_name_category(name: &str, category: &str) -> CustomServiceDefinition {
        let mut definition = CustomServiceDefinition::default();
        definition.base.name = name.to_string();
        definition.base.category = category.to_string();
        definition
    }

    #[test]
    fn accepts_valid_definition() {
        let mut definition = definition_with_name_category("My Service", "Database");
        assert!(
            CustomServiceDefinitionService::validate_custom_definition(&mut definition).is_ok()
        );
        // Name is trimmed by validation.
        let mut padded = definition_with_name_category("  My Service  ", "Database");
        assert!(CustomServiceDefinitionService::validate_custom_definition(&mut padded).is_ok());
        assert_eq!(padded.base.name, "My Service");
    }

    #[test]
    fn rejects_empty_name() {
        let mut definition = definition_with_name_category("   ", "Database");
        assert!(
            CustomServiceDefinitionService::validate_custom_definition(&mut definition).is_err()
        );
    }

    #[test]
    fn rejects_invalid_category() {
        let mut definition = definition_with_name_category("My Service", "NotACategory");
        let err = CustomServiceDefinitionService::validate_custom_definition(&mut definition)
            .unwrap_err();
        assert!(err.to_string().contains("Invalid service category"));
    }

    #[test]
    fn rejects_builtin_name_collision() {
        let mut definition = definition_with_name_category("DNS Server", "Database");
        let err = CustomServiceDefinitionService::validate_custom_definition(&mut definition)
            .unwrap_err();
        assert!(
            err.to_string()
                .contains("conflicts with an existing built-in service")
        );
    }

    #[test]
    fn rejects_builtin_collision_case_insensitively() {
        let mut definition = definition_with_name_category("dns server", "Database");
        assert!(
            CustomServiceDefinitionService::validate_custom_definition(&mut definition).is_err()
        );
    }
}

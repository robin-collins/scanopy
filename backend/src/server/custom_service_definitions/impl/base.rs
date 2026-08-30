use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::shared::entities::ChangeTriggersTopologyStaleness;

/// The mutable fields of a custom service definition (everything except
/// id/created_at/updated_at).
///
/// Mirrors the surface of a built-in `ServiceDefinition` (name, description,
/// category, logo_url, logo_needs_white_background, is_generic). The
/// `discovery_pattern` a built-in carries is deliberately out of scope here:
/// custom entries extend the *catalogue* and participate in manual
/// classification, not automatic detection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default, Validate, ToSchema)]
pub struct CustomServiceDefinitionBase {
    /// Service id shown in pickers and stored in `services.service_definition`.
    /// Must not collide (case-insensitively) with a built-in definition id.
    #[validate(length(
        min = 1,
        max = 39,
        message = "Name must be between 1 and 39 characters"
    ))]
    pub name: String,
    /// Service description. < 100 characters, matching built-ins.
    #[validate(length(max = 100, message = "Description must be at most 100 characters"))]
    pub description: String,
    /// A valid `ServiceCategory` id (e.g. "Database", "Media"). No FK — the
    /// backend validates the string against the `ServiceCategory` enum.
    pub category: String,
    /// URL of icon, or a static path when serving from `/logos`.
    #[validate(length(max = 2048, message = "Logo URL must be at most 2048 characters"))]
    pub logo_url: String,
    /// Whether the logo only has a dark variant / needs a white background.
    pub logo_needs_white_background: bool,
    /// Whether this service is not tied to a particular brand or vendor.
    pub is_generic: bool,
}

/// A user-created service definition extending the built-in catalogue.
///
/// Built-in service definitions are compile-time Rust types and have no rows
/// here, so "built-in is read-only" is automatic: every row in this table is
/// a custom entry with full CRUD.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default, ToSchema, Validate)]
pub struct CustomServiceDefinition {
    /// Server-assigned unique identifier.
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    /// When this definition was created.
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    /// When this definition was last modified.
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: CustomServiceDefinitionBase,
}

impl Display for CustomServiceDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (ID: {})", self.base.name, self.id)
    }
}

impl ChangeTriggersTopologyStaleness<CustomServiceDefinition> for CustomServiceDefinition {
    fn triggers_staleness(&self, _other: Option<CustomServiceDefinition>) -> bool {
        // A catalogue entry is referenced by name string; topology rendering
        // resolves missing names to a fallback, so nothing needs a rebuild.
        false
    }
}

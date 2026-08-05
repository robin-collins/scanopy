use chrono::{DateTime, Utc};
use lucide_icons::Icon;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::shared::{entities::ChangeTriggersTopologyStaleness, types::Color};

/// The base data for a LibraryObject entity (everything except id/created_at/updated_at).
///
/// No `PartialEq`/`Eq`/`Hash` here (unlike most `*Base` structs) — `icon`'s
/// type (`lucide_icons::Icon`) doesn't implement them.
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema, Default)]
pub struct LibraryObjectBase {
    /// `None` marks a seeded built-in stencil, shared read-only across every
    /// organization. `Some(org_id)` is an organization's own addition to the
    /// palette. See `LibraryObjectService` for the update/delete guard that
    /// keeps built-ins from being edited.
    #[schema(read_only)]
    pub organization_id: Option<Uuid>,
    /// Human-facing name for this stencil (e.g. "Router", "Firewall", "Cloud").
    #[validate(length(
        min = 1,
        max = 100,
        message = "Name must be between 1 and 100 characters"
    ))]
    pub name: String,
    /// Kebab-case lucide icon name, used when no uploaded image is set.
    #[schema(value_type = Option<String>)]
    pub icon: Option<Icon>,
    pub color: Option<Color>,
    /// Path relative to the server's configured data directory. Set only when
    /// an image has been uploaded for this object (overrides `icon`).
    #[schema(read_only)]
    pub storage_path: Option<String>,
    /// MIME type sniffed from the uploaded bytes, when `storage_path` is set.
    #[schema(read_only)]
    pub content_type: Option<String>,
    /// Size in bytes of the uploaded image, when `storage_path` is set.
    #[schema(read_only)]
    pub size_bytes: Option<i64>,
}

/// A stencil in the custom-topology-view object palette (router, switch,
/// firewall, cloud, or an organization's own addition).
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema, Validate)]
pub struct LibraryObject {
    /// Server-assigned unique identifier.
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    /// When this stencil was created.
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    /// When this stencil was last modified.
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: LibraryObjectBase,
}

impl Display for LibraryObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (ID: {})", self.base.name, self.id)
    }
}

/// Manual impl: `lucide_icons::Icon` doesn't implement `PartialEq`, so it's
/// compared via its `Display` string instead. Required by the top-level
/// `Entity` enum's derived `PartialEq`.
impl PartialEq for LibraryObjectBase {
    fn eq(&self, other: &Self) -> bool {
        self.organization_id == other.organization_id
            && self.name == other.name
            && self.icon.map(|i| i.to_string()) == other.icon.map(|i| i.to_string())
            && self.color == other.color
            && self.storage_path == other.storage_path
            && self.content_type == other.content_type
            && self.size_bytes == other.size_bytes
    }
}

impl PartialEq for LibraryObject {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.created_at == other.created_at
            && self.updated_at == other.updated_at
            && self.base == other.base
    }
}

impl ChangeTriggersTopologyStaleness<LibraryObject> for LibraryObject {
    fn triggers_staleness(&self, _other: Option<LibraryObject>) -> bool {
        false
    }
}

use chrono::{DateTime, Utc};
use lucide_icons::Icon;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::shared::{entities::ChangeTriggersTopologyStaleness, types::Color};

/// The base data for a Category entity (everything except id/created_at/updated_at).
///
/// No `PartialEq`/`Eq`/`Hash`/`Default` derive here (unlike most `*Base`
/// structs) — `icon`'s type (`lucide_icons::Icon`) doesn't implement them.
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CategoryBase {
    /// `None` marks a seeded built-in category (Router, Switch, WiFi AP, ...),
    /// shared read-only across every organization. `Some(org_id)` is an
    /// organization's own addition. See `CategoryService` for the
    /// update/delete guard that keeps built-ins from being edited.
    #[schema(read_only)]
    pub organization_id: Option<Uuid>,
    #[validate(length(
        min = 1,
        max = 100,
        message = "Name must be between 1 and 100 characters"
    ))]
    pub name: String,
    pub description: Option<String>,
    pub color: Color,
    /// Kebab-case lucide icon name.
    #[schema(value_type = String)]
    pub icon: Icon,
    /// Scan-planning hints the daemon reads when a host is assigned this
    /// category (see `HostScanHints`). `true` downgrades what would've been
    /// a full 65k-port scan down to the network's light port set (plus
    /// `preferred_ports`, if any) for any host in this category.
    pub skip_full_port_scan: bool,
    /// Always include these ports when scanning a host in this category,
    /// even during a light scan.
    pub preferred_ports: Option<Vec<u16>>,
}

/// A device category assignable to a host (Router, Switch, WiFi AP, Printer,
/// or an organization's own addition), also used as a scan-planning hint by
/// the discovery daemon.
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema, Validate)]
pub struct Category {
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: CategoryBase,
}

impl Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (ID: {})", self.base.name, self.id)
    }
}

/// Manual impl: `lucide_icons::Icon` doesn't implement `PartialEq`, so it's
/// compared via its `Display` string instead. Required by the top-level
/// `Entity` enum's derived `PartialEq`.
impl PartialEq for CategoryBase {
    fn eq(&self, other: &Self) -> bool {
        self.organization_id == other.organization_id
            && self.name == other.name
            && self.description == other.description
            && self.color == other.color
            && self.icon.to_string() == other.icon.to_string()
            && self.skip_full_port_scan == other.skip_full_port_scan
            && self.preferred_ports == other.preferred_ports
    }
}

impl PartialEq for Category {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.created_at == other.created_at
            && self.updated_at == other.updated_at
            && self.base == other.base
    }
}

/// Manual impl: `lucide_icons::Icon` doesn't implement `Default`, so
/// `#[derive(Default)]` can't be used here. Required by `Category`'s own
/// `#[derive(Default)]` (used by `EntityDiscriminants`'s default-construction
/// match in `shared/entities.rs`).
impl Default for CategoryBase {
    fn default() -> Self {
        Self {
            organization_id: None,
            name: String::new(),
            description: None,
            color: Color::default(),
            icon: Icon::CircleQuestionMark,
            skip_full_port_scan: false,
            preferred_ports: None,
        }
    }
}

impl ChangeTriggersTopologyStaleness<Category> for Category {
    fn triggers_staleness(&self, _other: Option<Category>) -> bool {
        // A category rename/re-icon doesn't change what a host IS, but hosts
        // reference categories by id (not denormalized name), and topology
        // node rendering reads the category live — no rebuild needed.
        false
    }
}

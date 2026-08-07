use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::shared::{entities::ChangeTriggersTopologyStaleness, types::Color};

fn default_true() -> bool {
    true
}

fn default_grid_size() -> i64 {
    20
}

/// The base data for a CustomTopologyView entity (everything except id/created_at/updated_at).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Validate, ToSchema)]
pub struct CustomTopologyViewBase {
    /// The network this view belongs to.
    pub network_id: Uuid,
    /// Human-facing name for this view, shown in the view switcher.
    #[validate(length(
        min = 1,
        max = 100,
        message = "Name must be between 1 and 100 characters"
    ))]
    pub name: String,
    /// Free-text description of what this view represents.
    #[validate(length(max = 2000, message = "Description is too long"))]
    pub description: Option<String>,
    /// Canvas background colour.
    pub background_color: Option<Color>,
    /// Whether the dotted background grid is shown.
    #[serde(default = "default_true")]
    pub show_grid: bool,
    /// Spacing of the background grid / drag snap increment, in pixels.
    #[validate(range(
        min = 5,
        max = 200,
        message = "Grid size must be between 5 and 200 pixels"
    ))]
    #[serde(default = "default_grid_size")]
    pub grid_size: i64,
    /// Whether dragged nodes snap to the grid.
    #[serde(default = "default_true")]
    pub snap_to_grid: bool,
    /// Default font family for new text-bearing objects on this canvas — a
    /// curated Google Font id, falling back to the safe system stack when unset.
    #[validate(length(max = 100, message = "Font family is too long"))]
    pub default_font_family: Option<String>,
    /// Default font size for new text-bearing objects on this canvas, in pixels.
    #[validate(range(
        min = 10,
        max = 72,
        message = "Font size must be between 10 and 72 pixels"
    ))]
    pub default_font_size: Option<i64>,
    /// Default primary colour for newly created objects.
    pub default_primary_color: Option<Color>,
    /// Default colour for newly created connectors/edges.
    pub default_connector_color: Option<Color>,
}

impl Default for CustomTopologyViewBase {
    fn default() -> Self {
        Self {
            network_id: Uuid::nil(),
            name: "New View".to_string(),
            description: None,
            background_color: None,
            show_grid: true,
            grid_size: 20,
            snap_to_grid: true,
            default_font_family: None,
            default_font_size: None,
            default_primary_color: None,
            default_connector_color: None,
        }
    }
}

/// A user-authored topology view: unlike the built-in L2/L3/Workloads/
/// Application views (computed live from entity data), a custom view's nodes
/// and edges (`CustomViewNode`/`CustomViewEdge`) are hand-placed by the user
/// and persisted as-is.
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
pub struct CustomTopologyView {
    /// Server-assigned unique identifier.
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    /// When this view was created.
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    /// When this view was last modified.
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: CustomTopologyViewBase,
}

impl Display for CustomTopologyView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (ID: {})", self.base.name, self.id)
    }
}

impl ChangeTriggersTopologyStaleness<CustomTopologyView> for CustomTopologyView {
    fn triggers_staleness(&self, _other: Option<CustomTopologyView>) -> bool {
        false
    }
}

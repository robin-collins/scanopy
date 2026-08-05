use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::shared::{entities::ChangeTriggersTopologyStaleness, types::Color};

/// The base data for a CustomViewEdge entity (everything except id/created_at/updated_at).
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, Validate, ToSchema,
)]
pub struct CustomViewEdgeBase {
    /// The custom topology view this edge is drawn on.
    pub view_id: Uuid,
    /// Denormalized from the parent view's network_id — see the migration
    /// comment; required for the generic network-scoped access-control filter.
    pub network_id: Uuid,
    /// The node this edge is drawn from.
    pub source_node_id: Uuid,
    /// The node this edge is drawn to.
    pub target_node_id: Uuid,
    /// Which handle on the source node the edge was dragged from (e.g.
    /// `"handle-Top"`) — re-rendering needs this since nodes expose one
    /// handle per side.
    #[validate(length(max = 40, message = "Handle id is too long"))]
    pub source_handle: Option<String>,
    /// Which handle on the target node the edge was dragged to, for the same
    /// re-rendering reason as `source_handle`.
    #[validate(length(max = 40, message = "Handle id is too long"))]
    pub target_handle: Option<String>,
    /// Optional text label shown on the edge.
    #[validate(length(max = 200, message = "Label is too long"))]
    pub label: Option<String>,
    pub color: Option<Color>,
    /// Marks this join as a dependency rather than a generic link.
    #[serde(default)]
    pub is_dependency: bool,
    /// Optional URL opened when the join label is activated.
    #[validate(length(max = 2048, message = "Link is too long"))]
    pub link_url: Option<String>,
}

/// A manually drawn edge between two nodes on the same custom topology view.
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
pub struct CustomViewEdge {
    /// Server-assigned unique identifier.
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    /// When this edge was created.
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    /// When this edge was last modified.
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: CustomViewEdgeBase,
}

impl Display for CustomViewEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Edge {} -> {} (ID: {})",
            self.base.source_node_id, self.base.target_node_id, self.id
        )
    }
}

impl ChangeTriggersTopologyStaleness<CustomViewEdge> for CustomViewEdge {
    fn triggers_staleness(&self, _other: Option<CustomViewEdge>) -> bool {
        false
    }
}

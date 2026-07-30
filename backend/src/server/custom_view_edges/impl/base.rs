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
    pub view_id: Uuid,
    /// Denormalized from the parent view's network_id — see the migration
    /// comment; required for the generic network-scoped access-control filter.
    pub network_id: Uuid,
    pub source_node_id: Uuid,
    pub target_node_id: Uuid,
    /// Which handle on the source node the edge was dragged from (e.g.
    /// `"handle-Top"`) — re-rendering needs this since nodes expose one
    /// handle per side.
    #[validate(length(max = 40, message = "Handle id is too long"))]
    pub source_handle: Option<String>,
    #[validate(length(max = 40, message = "Handle id is too long"))]
    pub target_handle: Option<String>,
    #[validate(length(max = 200, message = "Label is too long"))]
    pub label: Option<String>,
    pub color: Option<Color>,
}

/// A manually drawn edge between two nodes on the same custom topology view.
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
pub struct CustomViewEdge {
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

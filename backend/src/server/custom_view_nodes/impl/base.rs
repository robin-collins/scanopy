use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::custom_view_nodes::r#impl::types::{CornerStyle, NodeKind, NodeStyle};
use crate::server::shared::{
    entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants},
    types::Color,
};

/// The base data for a CustomViewNode entity (everything except id/created_at/updated_at).
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, Validate, ToSchema,
)]
pub struct CustomViewNodeBase {
    pub view_id: Uuid,
    /// Denormalized from the parent view's network_id — see the migration
    /// comment; required for the generic network-scoped access-control filter.
    pub network_id: Uuid,
    pub kind: NodeKind,
    /// `kind = Entity` only.
    pub entity_id: Option<Uuid>,
    /// `kind = Entity` only.
    pub entity_type: Option<EntityDiscriminants>,
    /// `kind = Library` only.
    pub library_object_id: Option<Uuid>,
    /// `kind = Text` only — the annotation body.
    #[validate(length(max = 5000, message = "Text is too long"))]
    pub text_content: Option<String>,
    /// Display-label override for `Entity`/`Library` nodes, or the frame name
    /// for `kind = Group`.
    #[validate(length(max = 200, message = "Label is too long"))]
    pub label: Option<String>,
    /// `Entity`/`Library` nodes only.
    pub style: Option<NodeStyle>,
    /// Override for the `Badge` style; defaults to the first 1-2 letters of
    /// the label on the frontend when unset.
    #[validate(length(max = 2, message = "Badge text must be at most 2 characters"))]
    pub badge_text: Option<String>,
    /// `kind = Group` frames only.
    pub color: Option<Color>,
    /// `kind = Group` frames only.
    pub corner_style: Option<CornerStyle>,
    /// Set when this node is dragged inside a `Group` frame.
    pub parent_node_id: Option<Uuid>,
    pub x: i64,
    pub y: i64,
    /// `kind = Group` frames only.
    pub width: Option<i64>,
    /// `kind = Group` frames only.
    pub height: Option<i64>,
    /// Per-node uploaded image, overriding the entity's/library object's own
    /// image when set. Path relative to the server's configured data directory.
    #[schema(read_only)]
    pub storage_path: Option<String>,
    #[schema(read_only)]
    pub content_type: Option<String>,
    #[schema(read_only)]
    pub size_bytes: Option<i64>,
}

/// A node placed on a custom topology view's canvas.
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
pub struct CustomViewNode {
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
    pub base: CustomViewNodeBase,
}

impl Display for CustomViewNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} node (ID: {})", self.base.kind, self.id)
    }
}

impl ChangeTriggersTopologyStaleness<CustomViewNode> for CustomViewNode {
    fn triggers_staleness(&self, _other: Option<CustomViewNode>) -> bool {
        false
    }
}

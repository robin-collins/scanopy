use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::custom_view_nodes::r#impl::types::{
    BorderStyle, CornerStyle, NodeKind, NodeStyle, TextAlign,
};
use crate::server::shared::{
    entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants},
    types::Color,
};

fn default_true() -> bool {
    true
}

/// The base data for a CustomViewNode entity (everything except id/created_at/updated_at).
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, Validate, ToSchema,
)]
pub struct CustomViewNodeBase {
    /// The custom topology view this node is placed on.
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
    /// Font family used by this object's text or label — a curated Google
    /// Font id (e.g. "Inter", "Roboto Mono") or `None` for the safe system
    /// fallback stack. Free-form rather than a fixed enum so the curated
    /// catalog can grow without a migration.
    #[validate(length(max = 100, message = "Font family is too long"))]
    pub font_family: Option<String>,
    /// Font size used by this object's text or label, in pixels.
    #[validate(range(min = 10, message = "Font size must be at least 10 pixels"))]
    pub font_size: Option<i64>,
    /// Bold emphasis used by this object's text or label.
    #[serde(default)]
    pub font_bold: bool,
    /// Italic emphasis used by this object's text or label.
    #[serde(default)]
    pub font_italic: bool,
    /// Underline emphasis used by this object's text or label.
    #[serde(default)]
    pub font_underline: bool,
    /// Horizontal text alignment used by this object's text or label.
    pub text_align: Option<TextAlign>,
    /// Display-label override for `Entity`/`Library` nodes, or the frame's
    /// visible on-canvas text for `kind = Group`.
    #[validate(length(max = 200, message = "Label is too long"))]
    pub label: Option<String>,
    /// `kind = Group` only — internal/organizational name for the frame,
    /// distinct from `label` (the visible on-canvas text).
    #[validate(length(max = 200, message = "Name is too long"))]
    pub name: Option<String>,
    /// `kind = Group` only — a longer free-text description of the frame's
    /// purpose.
    #[validate(length(max = 2000, message = "Description is too long"))]
    pub description: Option<String>,
    /// `kind = Group` only — whether the label is rendered on the canvas.
    #[serde(default = "default_true")]
    pub show_label: bool,
    /// `kind = Group` only — whether the description is rendered on the canvas.
    #[serde(default = "default_true")]
    pub show_description: bool,
    /// `Entity`/`Library` nodes only.
    pub style: Option<NodeStyle>,
    /// Override for the `Badge` style; defaults to the first 1-2 letters of
    /// the label on the frontend when unset.
    #[validate(length(max = 2, message = "Badge text must be at most 2 characters"))]
    pub badge_text: Option<String>,
    /// Legacy primary color. New clients also populate `primary_color`.
    pub color: Option<Color>,
    pub primary_color: Option<Color>,
    pub secondary_color: Option<Color>,
    pub background_color: Option<Color>,
    /// Object opacity as a percentage from fully transparent (0) to opaque (100).
    #[validate(range(min = 0, max = 100, message = "Opacity must be between 0 and 100"))]
    pub opacity: Option<i64>,
    /// Corner treatment shared by all object kinds.
    pub corner_style: Option<CornerStyle>,
    /// Border treatment shared by all object kinds.
    pub border_style: Option<BorderStyle>,
    /// Optional URL opened when the object is activated.
    #[validate(length(max = 2048, message = "Link is too long"))]
    pub link_url: Option<String>,
    /// Set when this node is dragged inside a `Group` frame.
    pub parent_node_id: Option<Uuid>,
    /// Horizontal position on the canvas, in pixels.
    pub x: i64,
    /// Vertical position on the canvas, in pixels.
    pub y: i64,
    /// Persisted width for horizontal stretch/scale.
    pub width: Option<i64>,
    /// Persisted height for vertical stretch/scale.
    pub height: Option<i64>,
    /// Per-node uploaded image, overriding the entity's/library object's own
    /// image when set. Path relative to the server's configured data directory.
    #[schema(read_only)]
    pub storage_path: Option<String>,
    /// MIME type of the uploaded image, when `storage_path` is set.
    #[schema(read_only)]
    pub content_type: Option<String>,
    /// Size in bytes of the uploaded image, when `storage_path` is set.
    #[schema(read_only)]
    pub size_bytes: Option<i64>,
}

/// A node placed on a custom topology view's canvas.
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
pub struct CustomViewNode {
    /// Server-assigned unique identifier.
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    /// When this node was created.
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    /// When this node was last modified.
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

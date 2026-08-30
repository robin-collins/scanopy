use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};
use utoipa::ToSchema;

/// What a `CustomViewNode` represents on the canvas.
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    EnumIter,
    IntoStaticStr,
    Display,
    EnumString,
    Default,
    ToSchema,
)]
pub enum NodeKind {
    /// References a real inventory entity via `entity_id` + `entity_type`.
    #[default]
    Entity,
    /// References a `LibraryObject` stencil via `library_object_id`.
    Library,
    /// A freeform text annotation (`text_content`).
    Text,
    /// A colored named frame other nodes can be parented under.
    Group,
}

/// How an `Entity`/`Library` node renders. `StatsCard` is only meaningful for
/// `Entity` nodes referencing a `Host` — validated at the service layer, not
/// the database.
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    EnumIter,
    IntoStaticStr,
    Display,
    EnumString,
    Default,
    ToSchema,
)]
pub enum NodeStyle {
    #[default]
    Image,
    ImageBordered,
    Badge,
    StatsCard,
}

/// Frame corner treatment for `Group` nodes.
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    EnumIter,
    IntoStaticStr,
    Display,
    EnumString,
    Default,
    ToSchema,
)]
pub enum CornerStyle {
    #[default]
    Rounded,
    Square,
}

/// Horizontal text alignment applied to any custom-canvas object's label or text.
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    EnumIter,
    IntoStaticStr,
    Display,
    EnumString,
    Default,
    ToSchema,
)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// Border treatment shared by every object placed on a custom canvas.
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    EnumIter,
    IntoStaticStr,
    Display,
    EnumString,
    Default,
    ToSchema,
)]
pub enum BorderStyle {
    None,
    #[default]
    Solid,
    Dashed,
    Dotted,
    Double,
}

/// Placement of a Service entity's detected or custom icon.
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    EnumIter,
    IntoStaticStr,
    Display,
    EnumString,
    Default,
    ToSchema,
)]
pub enum ServiceIconPosition {
    #[default]
    BeforeName,
    AfterName,
    Center,
}

/// Vertical anchor for the label on a custom-canvas Service entity.
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    EnumIter,
    IntoStaticStr,
    Display,
    EnumString,
    Default,
    ToSchema,
)]
pub enum ServiceLabelVerticalAlign {
    Top,
    Middle,
    #[default]
    Bottom,
}

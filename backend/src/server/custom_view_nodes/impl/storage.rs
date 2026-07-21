use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{Row, postgres::PgRow};
use uuid::Uuid;

use crate::server::{
    custom_view_nodes::r#impl::base::{CustomViewNode, CustomViewNodeBase},
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::traits::{Entity, SqlValue, Storable},
        types::Color,
    },
};

/// CSV row representation for CustomViewNode export
#[derive(Serialize)]
pub struct CustomViewNodeCsvRow {
    pub id: Uuid,
    pub view_id: Uuid,
    pub network_id: Uuid,
    pub kind: String,
    pub label: Option<String>,
    pub x: i64,
    pub y: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for CustomViewNode {
    type BaseData = CustomViewNodeBase;

    fn table_name() -> &'static str {
        "custom_topology_view_nodes"
    }

    fn new(base: Self::BaseData) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base,
        }
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>), anyhow::Error> {
        Ok((
            vec![
                "id",
                "view_id",
                "network_id",
                "kind",
                "entity_id",
                "entity_type",
                "library_object_id",
                "text_content",
                "label",
                "style",
                "badge_text",
                "color",
                "corner_style",
                "parent_node_id",
                "x",
                "y",
                "width",
                "height",
                "storage_path",
                "content_type",
                "size_bytes",
                "created_at",
                "updated_at",
            ],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::Uuid(self.base.view_id),
                SqlValue::Uuid(self.base.network_id),
                SqlValue::String(self.base.kind.to_string()),
                SqlValue::OptionalUuid(self.base.entity_id),
                SqlValue::OptionalString(
                    self.base
                        .entity_type
                        .map(|t| serde_json::to_string(&t).unwrap_or_default()),
                ),
                SqlValue::OptionalUuid(self.base.library_object_id),
                SqlValue::OptionalString(self.base.text_content.clone()),
                SqlValue::OptionalString(self.base.label.clone()),
                SqlValue::OptionalString(self.base.style.map(|s| s.to_string())),
                SqlValue::OptionalString(self.base.badge_text.clone()),
                SqlValue::OptionalString(self.base.color.map(|c| c.to_string())),
                SqlValue::OptionalString(self.base.corner_style.map(|c| c.to_string())),
                SqlValue::OptionalUuid(self.base.parent_node_id),
                SqlValue::I64(self.base.x),
                SqlValue::I64(self.base.y),
                SqlValue::OptionalI64(self.base.width),
                SqlValue::OptionalI64(self.base.height),
                SqlValue::OptionalString(self.base.storage_path.clone()),
                SqlValue::OptionalString(self.base.content_type.clone()),
                SqlValue::OptionalI64(self.base.size_bytes),
                SqlValue::Timestamp(self.created_at),
                SqlValue::Timestamp(self.updated_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        Ok(CustomViewNode {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: CustomViewNodeBase {
                view_id: row.get("view_id"),
                network_id: row.get("network_id"),
                kind: row.get::<String, _>("kind").parse().unwrap_or_default(),
                entity_id: row.get("entity_id"),
                entity_type: row
                    .get::<Option<String>, _>("entity_type")
                    .and_then(|s| serde_json::from_str(&s).ok()),
                library_object_id: row.get("library_object_id"),
                text_content: row.get("text_content"),
                label: row.get("label"),
                style: row
                    .get::<Option<String>, _>("style")
                    .and_then(|s| s.parse().ok()),
                badge_text: row.get("badge_text"),
                color: row
                    .get::<Option<String>, _>("color")
                    .and_then(|s| s.parse::<Color>().ok()),
                corner_style: row
                    .get::<Option<String>, _>("corner_style")
                    .and_then(|s| s.parse().ok()),
                parent_node_id: row.get("parent_node_id"),
                x: row.get("x"),
                y: row.get("y"),
                width: row.get("width"),
                height: row.get("height"),
                storage_path: row.get("storage_path"),
                content_type: row.get("content_type"),
                size_bytes: row.get("size_bytes"),
            },
        })
    }
}

impl Entity for CustomViewNode {
    fn id(&self) -> Uuid {
        self.id
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    fn set_id(&mut self, id: Uuid) {
        self.id = id;
    }

    fn set_created_at(&mut self, time: DateTime<Utc>) {
        self.created_at = time;
    }

    type CsvRow = CustomViewNodeCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        CustomViewNodeCsvRow {
            id: self.id,
            view_id: self.base.view_id,
            network_id: self.base.network_id,
            kind: self.base.kind.to_string(),
            label: self.base.label.clone(),
            x: self.base.x,
            y: self.base.y,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::CustomViewNode
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Custom View Node";
    const ENTITY_NAME_PLURAL: &'static str = "Custom View Nodes";
    const ENTITY_DESCRIPTION: &'static str = "A node placed on a custom topology view's canvas — an inventory entity, a library object stencil, a text annotation, or a group frame.";

    fn entity_category() -> EntityCategory {
        EntityCategory::Visualization
    }

    fn network_id(&self) -> Option<Uuid> {
        Some(self.base.network_id)
    }

    fn organization_id(&self) -> Option<Uuid> {
        None
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn set_updated_at(&mut self, time: DateTime<Utc>) {
        self.updated_at = time;
    }
}

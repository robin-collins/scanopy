use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{Row, postgres::PgRow};
use uuid::Uuid;

use crate::server::{
    custom_view_edges::r#impl::base::{CustomViewEdge, CustomViewEdgeBase},
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::traits::{Entity, SqlValue, Storable},
        types::Color,
    },
};

/// CSV row representation for CustomViewEdge export
#[derive(Serialize)]
pub struct CustomViewEdgeCsvRow {
    pub id: Uuid,
    pub view_id: Uuid,
    pub network_id: Uuid,
    pub source_node_id: Uuid,
    pub target_node_id: Uuid,
    pub label: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for CustomViewEdge {
    type BaseData = CustomViewEdgeBase;

    fn table_name() -> &'static str {
        "custom_topology_view_edges"
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
                "source_node_id",
                "target_node_id",
                "source_handle",
                "target_handle",
                "label",
                "color",
                "is_dependency",
                "link_url",
                "created_at",
                "updated_at",
            ],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::Uuid(self.base.view_id),
                SqlValue::Uuid(self.base.network_id),
                SqlValue::Uuid(self.base.source_node_id),
                SqlValue::Uuid(self.base.target_node_id),
                SqlValue::OptionalString(self.base.source_handle.clone()),
                SqlValue::OptionalString(self.base.target_handle.clone()),
                SqlValue::OptionalString(self.base.label.clone()),
                SqlValue::OptionalString(self.base.color.map(|c| c.to_string())),
                SqlValue::Bool(self.base.is_dependency),
                SqlValue::OptionalString(self.base.link_url.clone()),
                SqlValue::Timestamp(self.created_at),
                SqlValue::Timestamp(self.updated_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        Ok(CustomViewEdge {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: CustomViewEdgeBase {
                view_id: row.get("view_id"),
                network_id: row.get("network_id"),
                source_node_id: row.get("source_node_id"),
                target_node_id: row.get("target_node_id"),
                source_handle: row.get("source_handle"),
                target_handle: row.get("target_handle"),
                label: row.get("label"),
                color: row
                    .get::<Option<String>, _>("color")
                    .and_then(|s| s.parse::<Color>().ok()),
                is_dependency: row.get("is_dependency"),
                link_url: row.get("link_url"),
            },
        })
    }
}

impl Entity for CustomViewEdge {
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

    type CsvRow = CustomViewEdgeCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        CustomViewEdgeCsvRow {
            id: self.id,
            view_id: self.base.view_id,
            network_id: self.base.network_id,
            source_node_id: self.base.source_node_id,
            target_node_id: self.base.target_node_id,
            label: self.base.label.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::CustomViewEdge
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Custom View Edge";
    const ENTITY_NAME_PLURAL: &'static str = "Custom View Edges";
    const ENTITY_DESCRIPTION: &'static str =
        "A manually drawn edge between two nodes on a custom topology view.";

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

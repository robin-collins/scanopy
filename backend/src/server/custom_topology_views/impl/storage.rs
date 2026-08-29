use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{Row, postgres::PgRow};
use uuid::Uuid;

use crate::server::{
    custom_topology_views::r#impl::base::{CustomTopologyView, CustomTopologyViewBase},
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::traits::{Entity, SqlValue, Storable},
        types::Color,
    },
};

/// CSV row representation for CustomTopologyView export
#[derive(Serialize)]
pub struct CustomTopologyViewCsvRow {
    pub id: Uuid,
    pub network_id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for CustomTopologyView {
    type BaseData = CustomTopologyViewBase;

    fn table_name() -> &'static str {
        "custom_topology_views"
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
                "network_id",
                "name",
                "description",
                "background_color",
                "show_grid",
                "grid_size",
                "snap_to_grid",
                "default_font_family",
                "default_font_size",
                "default_text_color",
                "default_font_bold",
                "default_font_italic",
                "default_font_underline",
                "default_text_align",
                "default_primary_color",
                "default_connector_color",
                "created_at",
                "updated_at",
            ],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::Uuid(self.base.network_id),
                SqlValue::String(self.base.name.clone()),
                SqlValue::OptionalString(self.base.description.clone()),
                SqlValue::OptionalString(self.base.background_color.map(|c| c.to_string())),
                SqlValue::Bool(self.base.show_grid),
                SqlValue::I64(self.base.grid_size),
                SqlValue::Bool(self.base.snap_to_grid),
                SqlValue::OptionalString(self.base.default_font_family.clone()),
                SqlValue::OptionalI64(self.base.default_font_size),
                SqlValue::OptionalString(self.base.default_text_color.map(|c| c.to_string())),
                SqlValue::OptionalBool(self.base.default_font_bold),
                SqlValue::OptionalBool(self.base.default_font_italic),
                SqlValue::OptionalBool(self.base.default_font_underline),
                SqlValue::OptionalString(
                    self.base.default_text_align.map(|align| align.to_string()),
                ),
                SqlValue::OptionalString(self.base.default_primary_color.map(|c| c.to_string())),
                SqlValue::OptionalString(self.base.default_connector_color.map(|c| c.to_string())),
                SqlValue::Timestamp(self.created_at),
                SqlValue::Timestamp(self.updated_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        Ok(CustomTopologyView {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: CustomTopologyViewBase {
                network_id: row.get("network_id"),
                name: row.get("name"),
                description: row.get("description"),
                background_color: row
                    .get::<Option<String>, _>("background_color")
                    .and_then(|s| s.parse::<Color>().ok()),
                show_grid: row.get("show_grid"),
                grid_size: row.get("grid_size"),
                snap_to_grid: row.get("snap_to_grid"),
                default_font_family: row.get("default_font_family"),
                default_font_size: row.get("default_font_size"),
                default_text_color: row
                    .get::<Option<String>, _>("default_text_color")
                    .and_then(|s| s.parse::<Color>().ok()),
                default_font_bold: row.get("default_font_bold"),
                default_font_italic: row.get("default_font_italic"),
                default_font_underline: row.get("default_font_underline"),
                default_text_align: row
                    .get::<Option<String>, _>("default_text_align")
                    .and_then(|align| align.parse().ok()),
                default_primary_color: row
                    .get::<Option<String>, _>("default_primary_color")
                    .and_then(|s| s.parse::<Color>().ok()),
                default_connector_color: row
                    .get::<Option<String>, _>("default_connector_color")
                    .and_then(|s| s.parse::<Color>().ok()),
            },
        })
    }
}

impl Entity for CustomTopologyView {
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

    type CsvRow = CustomTopologyViewCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        CustomTopologyViewCsvRow {
            id: self.id,
            network_id: self.base.network_id,
            name: self.base.name.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::CustomTopologyView
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Custom Topology View";
    const ENTITY_NAME_PLURAL: &'static str = "Custom Topology Views";
    const ENTITY_DESCRIPTION: &'static str = "A user-authored topology view with hand-placed nodes and edges, distinct from the built-in live-computed views.";

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

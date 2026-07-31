use chrono::{DateTime, Utc};
use lucide_icons::Icon;
use serde::Serialize;
use sqlx::{Row, postgres::PgRow};
use uuid::Uuid;

use crate::server::{
    categories::r#impl::base::{Category, CategoryBase},
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::traits::{Entity, SqlValue, Storable},
        types::Color,
    },
};

/// CSV row representation for Category export
#[derive(Serialize)]
pub struct CategoryCsvRow {
    pub id: Uuid,
    pub organization_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub icon: String,
    pub skip_full_port_scan: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for Category {
    type BaseData = CategoryBase;

    fn table_name() -> &'static str {
        "categories"
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
                "organization_id",
                "name",
                "description",
                "color",
                "icon",
                "skip_full_port_scan",
                "preferred_ports",
                "created_at",
                "updated_at",
            ],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::OptionalUuid(self.base.organization_id),
                SqlValue::String(self.base.name.clone()),
                SqlValue::OptionalString(self.base.description.clone()),
                SqlValue::String(self.base.color.to_string()),
                SqlValue::String(self.base.icon.to_string()),
                SqlValue::Bool(self.base.skip_full_port_scan),
                SqlValue::OptionVecU16(self.base.preferred_ports.clone()),
                SqlValue::Timestamp(self.created_at),
                SqlValue::Timestamp(self.updated_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        let color_str: String = row.get("color");
        let icon_str: String = row.get("icon");
        let preferred_ports: Option<Vec<u16>> = row
            .try_get::<Option<serde_json::Value>, _>("preferred_ports")
            .ok()
            .flatten()
            .and_then(|v| serde_json::from_value(v).ok());

        Ok(Category {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: CategoryBase {
                organization_id: row.get("organization_id"),
                name: row.get("name"),
                description: row.get("description"),
                color: color_str.parse::<Color>().unwrap_or_default(),
                // `Icon::try_from`'s Err carries no data we can act on beyond
                // "not a recognized name" — degrade to a generic placeholder
                // rather than failing the whole row read.
                icon: Icon::try_from(icon_str.as_str()).unwrap_or(Icon::CircleQuestionMark),
                skip_full_port_scan: row.get("skip_full_port_scan"),
                preferred_ports,
            },
        })
    }
}

impl Entity for Category {
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

    type CsvRow = CategoryCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        CategoryCsvRow {
            id: self.id,
            organization_id: self.base.organization_id,
            name: self.base.name.clone(),
            description: self.base.description.clone(),
            color: self.base.color.to_string(),
            icon: self.base.icon.to_string(),
            skip_full_port_scan: self.base.skip_full_port_scan,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Category
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Category";
    const ENTITY_NAME_PLURAL: &'static str = "Categories";
    const ENTITY_DESCRIPTION: &'static str =
        "Device categories assignable to hosts (Router, Switch, WiFi AP, Printer, etc.), used as scan-planning hints by the discovery daemon.";

    fn entity_category() -> EntityCategory {
        EntityCategory::Metadata
    }

    fn network_id(&self) -> Option<Uuid> {
        None
    }

    fn organization_id(&self) -> Option<Uuid> {
        self.base.organization_id
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn set_updated_at(&mut self, time: DateTime<Utc>) {
        self.updated_at = time;
    }
}

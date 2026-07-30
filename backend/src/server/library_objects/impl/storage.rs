use chrono::{DateTime, Utc};
use lucide_icons::Icon;
use serde::Serialize;
use sqlx::{Row, postgres::PgRow};
use uuid::Uuid;

use crate::server::{
    library_objects::r#impl::base::{LibraryObject, LibraryObjectBase},
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::traits::{Entity, SqlValue, Storable},
        types::Color,
    },
};

/// CSV row representation for LibraryObject export
#[derive(Serialize)]
pub struct LibraryObjectCsvRow {
    pub id: Uuid,
    pub organization_id: Option<Uuid>,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for LibraryObject {
    type BaseData = LibraryObjectBase;

    fn table_name() -> &'static str {
        "library_objects"
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
                "icon",
                "color",
                "storage_path",
                "content_type",
                "size_bytes",
                "created_at",
                "updated_at",
            ],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::OptionalUuid(self.base.organization_id),
                SqlValue::String(self.base.name.clone()),
                SqlValue::OptionalString(self.base.icon.map(|i| i.to_string())),
                SqlValue::OptionalString(self.base.color.map(|c| c.to_string())),
                SqlValue::OptionalString(self.base.storage_path.clone()),
                SqlValue::OptionalString(self.base.content_type.clone()),
                SqlValue::OptionalI64(self.base.size_bytes),
                SqlValue::Timestamp(self.created_at),
                SqlValue::Timestamp(self.updated_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        Ok(LibraryObject {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: LibraryObjectBase {
                organization_id: row.get("organization_id"),
                name: row.get("name"),
                // `Icon::try_from`'s Err carries no data we can act on beyond
                // "not a recognized name" — degrade to no icon rather than
                // failing the whole row read.
                icon: row
                    .get::<Option<String>, _>("icon")
                    .and_then(|s| Icon::try_from(s.as_str()).ok()),
                color: row
                    .get::<Option<String>, _>("color")
                    .and_then(|s| s.parse::<Color>().ok()),
                storage_path: row.get("storage_path"),
                content_type: row.get("content_type"),
                size_bytes: row.get("size_bytes"),
            },
        })
    }
}

impl Entity for LibraryObject {
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

    type CsvRow = LibraryObjectCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        LibraryObjectCsvRow {
            id: self.id,
            organization_id: self.base.organization_id,
            name: self.base.name.clone(),
            icon: self.base.icon.map(|i| i.to_string()),
            color: self.base.color.map(|c| c.to_string()),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::LibraryObject
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Library Object";
    const ENTITY_NAME_PLURAL: &'static str = "Library Objects";
    const ENTITY_DESCRIPTION: &'static str = "Reusable stencils (router, switch, firewall, cloud, etc.) for the custom topology view object palette.";

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

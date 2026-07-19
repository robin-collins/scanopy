use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{Row, postgres::PgRow};
use uuid::Uuid;

use crate::server::{
    host_images::r#impl::base::{HostImage, HostImageBase},
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::traits::{Entity, SqlValue, Storable},
    },
};

/// CSV row representation for HostImage export
#[derive(Serialize)]
pub struct HostImageCsvRow {
    pub id: Uuid,
    pub host_id: Uuid,
    pub network_id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for HostImage {
    type BaseData = HostImageBase;

    fn table_name() -> &'static str {
        "host_images"
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
                "host_id",
                "network_id",
                "filename",
                "content_type",
                "size_bytes",
                "storage_path",
                "created_at",
                "updated_at",
            ],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::Uuid(self.base.host_id),
                SqlValue::Uuid(self.base.network_id),
                SqlValue::String(self.base.filename.clone()),
                SqlValue::String(self.base.content_type.clone()),
                SqlValue::I64(self.base.size_bytes),
                SqlValue::String(self.base.storage_path.clone()),
                SqlValue::Timestamp(self.created_at),
                SqlValue::Timestamp(self.updated_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        Ok(HostImage {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: HostImageBase {
                host_id: row.get("host_id"),
                network_id: row.get("network_id"),
                filename: row.get("filename"),
                content_type: row.get("content_type"),
                size_bytes: row.get("size_bytes"),
                storage_path: row.get("storage_path"),
            },
        })
    }
}

impl Entity for HostImage {
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

    type CsvRow = HostImageCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        HostImageCsvRow {
            id: self.id,
            host_id: self.base.host_id,
            network_id: self.base.network_id,
            filename: self.base.filename.clone(),
            content_type: self.base.content_type.clone(),
            size_bytes: self.base.size_bytes,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::HostImage
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Host Image";
    const ENTITY_NAME_PLURAL: &'static str = "Host Images";
    const ENTITY_DESCRIPTION: &'static str = "Uploaded images for a host's image gallery.";

    fn entity_category() -> EntityCategory {
        EntityCategory::NetworkInfrastructure
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

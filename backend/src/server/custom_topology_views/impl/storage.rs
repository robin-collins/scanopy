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
            vec!["id", "network_id", "name", "created_at", "updated_at"],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::Uuid(self.base.network_id),
                SqlValue::String(self.base.name.clone()),
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

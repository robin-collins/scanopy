use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{Row, postgres::PgRow};
use uuid::Uuid;

use crate::server::{
    custom_service_definitions::r#impl::base::{
        CustomServiceDefinition, CustomServiceDefinitionBase,
    },
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::traits::{Entity, SqlValue, Storable},
    },
};

/// CSV row representation for custom service definition export.
#[derive(Serialize)]
pub struct CustomServiceDefinitionCsvRow {
    pub id: Uuid,
    pub organization_id: Option<Uuid>,
    pub name: String,
    pub description: String,
    pub category: String,
    pub logo_url: String,
    pub logo_needs_white_background: bool,
    pub is_generic: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for CustomServiceDefinition {
    type BaseData = CustomServiceDefinitionBase;

    fn table_name() -> &'static str {
        "custom_service_definitions"
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
                "category",
                "logo_url",
                "logo_needs_white_background",
                "is_generic",
                "created_at",
                "updated_at",
            ],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::OptionalUuid(self.base.organization_id),
                SqlValue::String(self.base.name.clone()),
                SqlValue::String(self.base.description.clone()),
                SqlValue::String(self.base.category.clone()),
                SqlValue::String(self.base.logo_url.clone()),
                SqlValue::Bool(self.base.logo_needs_white_background),
                SqlValue::Bool(self.base.is_generic),
                SqlValue::Timestamp(self.created_at),
                SqlValue::Timestamp(self.updated_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        Ok(CustomServiceDefinition {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: CustomServiceDefinitionBase {
                organization_id: row.get("organization_id"),
                name: row.get("name"),
                description: row.get("description"),
                category: row.get("category"),
                logo_url: row.get("logo_url"),
                logo_needs_white_background: row.get("logo_needs_white_background"),
                is_generic: row.get("is_generic"),
            },
        })
    }
}

impl Entity for CustomServiceDefinition {
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

    type CsvRow = CustomServiceDefinitionCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        CustomServiceDefinitionCsvRow {
            id: self.id,
            organization_id: self.base.organization_id,
            name: self.base.name.clone(),
            description: self.base.description.clone(),
            category: self.base.category.clone(),
            logo_url: self.base.logo_url.clone(),
            logo_needs_white_background: self.base.logo_needs_white_background,
            is_generic: self.base.is_generic,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::CustomServiceDefinition
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Custom Service Definition";
    const ENTITY_NAME_PLURAL: &'static str = "Custom Service Definitions";
    const ENTITY_DESCRIPTION: &'static str = "User-created service definitions that extend the built-in service catalogue. Built-in definitions are compile-time and read-only; every row here is a custom entry with full CRUD.";

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

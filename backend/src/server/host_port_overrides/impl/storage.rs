use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{Row, postgres::PgRow};
use uuid::Uuid;

use crate::server::{
    host_port_overrides::r#impl::base::{HostPortOverride, HostPortOverrideBase, ServiceRefKind},
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::traits::{Entity, SqlValue, Storable},
    },
};

impl Storable for HostPortOverride {
    type BaseData = HostPortOverrideBase;

    fn table_name() -> &'static str {
        "host_port_overrides"
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
                "port_number",
                "port_protocol",
                "display_name",
                "icon_url",
                "service_ref_kind",
                "service_ref_id",
                "created_at",
                "updated_at",
            ],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::Uuid(self.base.host_id),
                SqlValue::Uuid(self.base.network_id),
                SqlValue::I32(self.base.port_number as i32),
                SqlValue::String(self.base.port_protocol.clone()),
                SqlValue::OptionalString(self.base.display_name.clone()),
                SqlValue::OptionalString(self.base.icon_url.clone()),
                SqlValue::OptionalString(self.base.service_ref_kind.map(|k| match k {
                    ServiceRefKind::BuiltIn => "BuiltIn".to_string(),
                    ServiceRefKind::Custom => "Custom".to_string(),
                })),
                SqlValue::OptionalString(self.base.service_ref_id.clone()),
                SqlValue::Timestamp(self.created_at),
                SqlValue::Timestamp(self.updated_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        Ok(HostPortOverride {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: HostPortOverrideBase {
                host_id: row.get("host_id"),
                network_id: row.get("network_id"),
                port_number: row.get::<i32, _>("port_number") as u16,
                port_protocol: row.get("port_protocol"),
                display_name: row.get("display_name"),
                icon_url: row.get("icon_url"),
                service_ref_kind: row
                    .get::<Option<String>, _>("service_ref_kind")
                    .map(|s| match s.as_str() {
                        "Custom" => ServiceRefKind::Custom,
                        _ => ServiceRefKind::BuiltIn,
                    }),
                service_ref_id: row.get("service_ref_id"),
            },
        })
    }
}

impl Entity for HostPortOverride {
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

    type CsvRow = HostPortOverrideCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        HostPortOverrideCsvRow {
            id: self.id,
            host_id: self.base.host_id,
            network_id: self.base.network_id,
            port_number: self.base.port_number,
            port_protocol: self.base.port_protocol.clone(),
            display_name: self.base.display_name.clone(),
            icon_url: self.base.icon_url.clone(),
            service_ref_kind: self.base.service_ref_kind.map(|k| match k {
                ServiceRefKind::BuiltIn => "BuiltIn".to_string(),
                ServiceRefKind::Custom => "Custom".to_string(),
            }),
            service_ref_id: self.base.service_ref_id.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::HostPortOverride
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Host Port Override";
    const ENTITY_NAME_PLURAL: &'static str = "Host Port Overrides";
    const ENTITY_DESCRIPTION: &'static str =
        "Per-host display override for a well-known or unclaimed port.";

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

/// CSV row representation for HostPortOverride export
#[derive(Serialize)]
pub struct HostPortOverrideCsvRow {
    pub id: Uuid,
    pub host_id: Uuid,
    pub network_id: Uuid,
    pub port_number: u16,
    pub port_protocol: String,
    pub display_name: Option<String>,
    pub icon_url: Option<String>,
    pub service_ref_kind: Option<String>,
    pub service_ref_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

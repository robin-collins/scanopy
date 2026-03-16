use chrono::{DateTime, Utc};
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    credentials::r#impl::{
        base::{Credential, CredentialBase},
        types::{CredentialType, SnmpVersion},
    },
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::traits::{Entity, SqlValue, Storable},
    },
};

/// CSV row representation for Credential export
#[derive(Serialize)]
pub struct CredentialCsvRow {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub credential_type: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for Credential {
    type BaseData = CredentialBase;

    fn table_name() -> &'static str {
        "credentials"
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

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>), anyhow::Error> {
        let Self {
            id,
            created_at,
            updated_at,
            base:
                Self::BaseData {
                    organization_id,
                    name,
                    credential_type,
                    tags: _, // Stored in entity_tags junction table
                },
        } = self.clone();

        // Build JSONB manually to avoid SecretString redaction in serde
        let credential_type_json = credential_type_to_json(&credential_type);

        Ok((
            vec![
                "id",
                "organization_id",
                "name",
                "credential_type",
                "created_at",
                "updated_at",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Uuid(organization_id),
                SqlValue::String(name),
                SqlValue::JsonValue(credential_type_json),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        let credential_type_json: serde_json::Value = row.get("credential_type");
        let credential_type = credential_type_from_json(credential_type_json)?;

        Ok(Credential {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: CredentialBase {
                organization_id: row.get("organization_id"),
                name: row.get("name"),
                credential_type,
                tags: Vec::new(), // Hydrated from entity_tags junction table
            },
        })
    }
}

/// Serialize CredentialType to JSON with secret fields exposed (for DB storage).
fn credential_type_to_json(ct: &CredentialType) -> serde_json::Value {
    match ct {
        CredentialType::Snmp { version, community } => {
            serde_json::json!({
                "type": "Snmp",
                "version": version,
                "community": community.expose_secret(),
            })
        }
        CredentialType::DockerProxyLocal {
            url,
            ssl_cert_path,
            ssl_key_path,
            ssl_chain_path,
        } => {
            serde_json::json!({
                "type": "DockerProxyLocal",
                "url": url,
                "ssl_cert_path": ssl_cert_path,
                "ssl_key_path": ssl_key_path,
                "ssl_chain_path": ssl_chain_path,
            })
        }
        CredentialType::DockerProxyRemote {
            url,
            ssl_cert,
            ssl_key,
            ssl_chain,
        } => {
            serde_json::json!({
                "type": "DockerProxyRemote",
                "url": url,
                "ssl_cert": ssl_cert,
                "ssl_key": ssl_key.as_ref().map(|s| s.expose_secret()),
                "ssl_chain": ssl_chain,
            })
        }
    }
}

/// Deserialize CredentialType from JSON, wrapping secret fields in SecretString.
fn credential_type_from_json(json: serde_json::Value) -> Result<CredentialType, anyhow::Error> {
    let obj = json
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("credential_type is not a JSON object"))?;

    let type_str = obj
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("credential_type missing 'type' field"))?;

    match type_str {
        "Snmp" => {
            let version_str = obj.get("version").and_then(|v| v.as_str()).unwrap_or("V2c");
            let version: SnmpVersion = version_str.parse().unwrap_or_default();
            let community = obj.get("community").and_then(|v| v.as_str()).unwrap_or("");
            Ok(CredentialType::Snmp {
                version,
                community: SecretString::from(community.to_string()),
            })
        }
        "DockerProxyLocal" => {
            let url = obj
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let ssl_cert_path = obj
                .get("ssl_cert_path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let ssl_key_path = obj
                .get("ssl_key_path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let ssl_chain_path = obj
                .get("ssl_chain_path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            Ok(CredentialType::DockerProxyLocal {
                url,
                ssl_cert_path,
                ssl_key_path,
                ssl_chain_path,
            })
        }
        "DockerProxyRemote" => {
            let url = obj
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let ssl_cert = obj
                .get("ssl_cert")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let ssl_key = obj
                .get("ssl_key")
                .and_then(|v| v.as_str())
                .map(|s| SecretString::from(s.to_string()));
            let ssl_chain = obj
                .get("ssl_chain")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            Ok(CredentialType::DockerProxyRemote {
                url,
                ssl_cert,
                ssl_key,
                ssl_chain,
            })
        }
        other => Err(anyhow::anyhow!("Unknown credential type: {}", other)),
    }
}

impl Entity for Credential {
    type CsvRow = CredentialCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        CredentialCsvRow {
            id: self.id,
            organization_id: self.base.organization_id,
            name: self.base.name.clone(),
            credential_type: self.base.credential_type.discriminant().to_string(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Credential
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Credential";
    const ENTITY_NAME_PLURAL: &'static str = "Credentials";
    const ENTITY_DESCRIPTION: &'static str = "Credentials for network device discovery and management. Supports SNMP, Docker proxy, and other credential types.";

    fn entity_category() -> EntityCategory {
        EntityCategory::DiscoveryAndDaemons
    }

    fn network_id(&self) -> Option<Uuid> {
        None
    }

    fn organization_id(&self) -> Option<Uuid> {
        Some(self.base.organization_id)
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn set_updated_at(&mut self, time: DateTime<Utc>) {
        self.updated_at = time;
    }

    fn get_tags(&self) -> Option<&Vec<Uuid>> {
        Some(&self.base.tags)
    }

    fn set_tags(&mut self, tags: Vec<Uuid>) {
        self.base.tags = tags;
    }
}

use crate::server::{
    ports::r#impl::base::PortType,
    shared::{
        entities::EntityDiscriminants,
        types::{
            Color, Icon,
            metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
        },
    },
};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize, Serializer, ser::SerializeMap};
use strum::VariantNames;
use strum_macros::EnumIter;
use utoipa::ToSchema;
use uuid::Uuid;

/// Serializer that redacts the secret value
fn redact_secret<S>(_secret: &SecretString, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str("********")
}

/// Secret value that can be either inline content or a file path on the daemon host.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "mode")]
pub enum SecretValue {
    Inline {
        #[serde(serialize_with = "redact_secret")]
        #[schema(value_type = String)]
        value: SecretString,
    },
    FilePath {
        path: String,
    },
}

fn default_docker_port() -> u16 {
    2376
}

/// SNMP protocol version
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash, Default, VariantNames, ToSchema,
)]
pub enum SnmpVersion {
    /// SNMPv2c (MVP - community string based)
    #[default]
    V2c,
    /// SNMPv3 (future - authentication + privacy)
    V3,
}

impl std::fmt::Display for SnmpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnmpVersion::V2c => write!(f, "V2c"),
            SnmpVersion::V3 => write!(f, "V3"),
        }
    }
}

impl std::str::FromStr for SnmpVersion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "V2C" | "2C" | "2" => Ok(SnmpVersion::V2c),
            "V3" | "3" => Ok(SnmpVersion::V3),
            _ => Err(anyhow::anyhow!("Invalid SNMP version: {}", s)),
        }
    }
}

/// Universal credential type — tagged enum stored as JSONB.
/// Each variant represents a different credential protocol/method.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum CredentialType {
    /// SNMP community string for querying network devices
    Snmp {
        #[serde(default)]
        version: SnmpVersion,
        community: SecretValue,
    },
    /// Docker API proxy credentials. Target IP determined from host interfaces at scan time.
    DockerProxy {
        /// Port for the Docker API proxy (default 2376)
        #[serde(default = "default_docker_port")]
        port: u16,
        /// Optional URL path prefix (e.g. "/v1.43")
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        /// PEM-encoded public certificate (always inline — not secret)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ssl_cert: Option<String>,
        /// Private key — inline PEM content or file path on daemon host
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ssl_key: Option<SecretValue>,
        /// PEM-encoded CA chain (always inline — not secret)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ssl_chain: Option<String>,
    },
}

impl PartialEq for CredentialType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Snmp {
                    version: v1,
                    community: c1,
                },
                Self::Snmp {
                    version: v2,
                    community: c2,
                },
            ) => {
                v1 == v2
                    && match (c1, c2) {
                        (SecretValue::Inline { value: a }, SecretValue::Inline { value: b }) => {
                            a.expose_secret() == b.expose_secret()
                        }
                        (SecretValue::FilePath { path: a }, SecretValue::FilePath { path: b }) => {
                            a == b
                        }
                        _ => false,
                    }
            }
            (
                Self::DockerProxy {
                    port: p1,
                    path: pa1,
                    ssl_cert: c1,
                    ssl_key: k1,
                    ssl_chain: ch1,
                },
                Self::DockerProxy {
                    port: p2,
                    path: pa2,
                    ssl_cert: c2,
                    ssl_key: k2,
                    ssl_chain: ch2,
                },
            ) => {
                p1 == p2
                    && pa1 == pa2
                    && c1 == c2
                    && match (k1, k2) {
                        (
                            Some(SecretValue::Inline { value: a }),
                            Some(SecretValue::Inline { value: b }),
                        ) => a.expose_secret() == b.expose_secret(),
                        (
                            Some(SecretValue::FilePath { path: a }),
                            Some(SecretValue::FilePath { path: b }),
                        ) => a == b,
                        (None, None) => true,
                        _ => false,
                    }
                    && ch1 == ch2
            }
            _ => false,
        }
    }
}

/// Returns a discriminant string for `CredentialType` (the serde tag value).
impl CredentialType {
    pub fn discriminant(&self) -> &'static str {
        match self {
            Self::Snmp { .. } => "Snmp",
            Self::DockerProxy { .. } => "DockerProxy",
        }
    }

    /// Ports that must be open on a host for this credential to be applicable during discovery.
    /// Empty vec means the credential applies regardless of open ports.
    /// When multiple ports are returned, the credential applies if *any* of them are open.
    pub fn required_ports(&self) -> Vec<PortType> {
        match self {
            Self::Snmp { .. } => vec![PortType::Snmp, PortType::SnmpAlt],
            Self::DockerProxy { port, .. } => vec![PortType::new_tcp(*port)],
        }
    }
}

// ============================================================================
// FieldDefinition — metadata for dynamic frontend form generation
// ============================================================================

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FieldDefinition {
    pub id: &'static str,
    pub label: &'static str,
    pub field_type: FieldType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<&'static str>,
    pub secret: bool,
    pub optional: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help_text: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<&'static [&'static str]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    String,
    Text,
    Select,
    SecretPathOrInline,
}

impl CredentialType {
    /// Returns field definitions for this credential type.
    /// Uses exhaustive destructuring for compile-time enforcement:
    /// adding a field to the enum variant without updating this method causes a compile error.
    pub fn field_definitions(&self) -> Vec<FieldDefinition> {
        match self {
            Self::Snmp {
                version: _,
                community: _,
            } => vec![
                FieldDefinition {
                    id: "version",
                    label: "SNMP Version",
                    field_type: FieldType::Select,
                    placeholder: None,
                    secret: false,
                    optional: false,
                    help_text: None,
                    options: Some(&["V2c"]),
                    default_value: Some("V2c"),
                },
                FieldDefinition {
                    id: "community",
                    label: "Community String",
                    field_type: FieldType::SecretPathOrInline,
                    placeholder: Some("e.g. custom-community-string"),
                    secret: true,
                    optional: false,
                    help_text: Some(
                        "Custom SNMP community string. The default 'public' community is always tried automatically during scans; add any additional community strings here.",
                    ),
                    options: None,
                    default_value: None,
                },
            ],
            Self::DockerProxy {
                port: _,
                path: _,
                ssl_cert: _,
                ssl_key: _,
                ssl_chain: _,
            } => vec![
                FieldDefinition {
                    id: "port",
                    label: "Docker API Port",
                    field_type: FieldType::String,
                    placeholder: Some("2376"),
                    secret: false,
                    optional: false,
                    help_text: Some(
                        "Docker API port on the target host. At scan time, the daemon connects to https://{host_ip}:{port}{path}",
                    ),
                    options: None,
                    default_value: Some("2376"),
                },
                FieldDefinition {
                    id: "path",
                    label: "URL Path Prefix",
                    field_type: FieldType::String,
                    placeholder: Some("/v1.43"),
                    secret: false,
                    optional: true,
                    help_text: Some("Optional URL path prefix appended after the port"),
                    options: None,
                    default_value: None,
                },
                FieldDefinition {
                    id: "ssl_cert",
                    label: "SSL Certificate",
                    field_type: FieldType::Text,
                    placeholder: Some("-----BEGIN CERTIFICATE-----"),
                    secret: false,
                    optional: true,
                    help_text: Some("PEM-encoded client certificate"),
                    options: None,
                    default_value: None,
                },
                FieldDefinition {
                    id: "ssl_key",
                    label: "SSL Private Key",
                    field_type: FieldType::SecretPathOrInline,
                    placeholder: None,
                    secret: true,
                    optional: true,
                    help_text: Some("PEM private key"),
                    options: None,
                    default_value: None,
                },
                FieldDefinition {
                    id: "ssl_chain",
                    label: "SSL CA Chain",
                    field_type: FieldType::Text,
                    placeholder: Some("-----BEGIN CERTIFICATE-----"),
                    secret: false,
                    optional: true,
                    help_text: Some("PEM-encoded CA certificate chain"),
                    options: None,
                    default_value: None,
                },
            ],
        }
    }
}

// ============================================================================
// Metadata trait implementations for fixture generation
// ============================================================================

/// Provide an iterator over all `CredentialType` variants (with default field values).
/// Used by `generate-fixtures` to produce `credential-types.json`.
#[derive(EnumIter)]
pub enum CredentialTypeVariant {
    Snmp,
    DockerProxy,
}

impl CredentialTypeVariant {
    pub fn to_credential_type(&self) -> CredentialType {
        match self {
            Self::Snmp => CredentialType::Snmp {
                version: SnmpVersion::default(),
                community: SecretValue::Inline {
                    value: SecretString::from(String::new()),
                },
            },
            Self::DockerProxy => CredentialType::DockerProxy {
                port: default_docker_port(),
                path: None,
                ssl_cert: None,
                ssl_key: None,
                ssl_chain: None,
            },
        }
    }
}

impl HasId for CredentialType {
    fn id(&self) -> &'static str {
        self.discriminant()
    }
}

impl EntityMetadataProvider for CredentialType {
    fn color(&self) -> Color {
        EntityDiscriminants::Credential.color()
    }
    fn icon(&self) -> Icon {
        EntityDiscriminants::Credential.icon()
    }
}

/// A credential assigned to a host, optionally limited to specific interfaces.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct CredentialAssignment {
    pub credential_id: Uuid,
    /// Interface IDs to limit this credential to. None = all host interfaces.
    #[serde(default)]
    #[schema(required)]
    pub interface_ids: Option<Vec<Uuid>>,
}

impl TypeMetadataProvider for CredentialType {
    fn name(&self) -> &'static str {
        match self {
            Self::Snmp { .. } => "SNMP",
            Self::DockerProxy { .. } => "Docker Proxy",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::Snmp { .. } => "SNMP community string for querying network devices",
            Self::DockerProxy { .. } => {
                "Docker API proxy credentials. The target IP is determined from the host's interfaces at scan time."
            }
        }
    }

    fn metadata(&self) -> serde_json::Value {
        serde_json::json!({ "fields": self.field_definitions() })
    }
}

// ============================================================================
// Storage serialization — exposes secrets for DB persistence
// ============================================================================

/// Wrapper that serializes `SecretValue` with secrets exposed (for DB storage).
struct StorageSecretValue<'a>(&'a SecretValue);

impl Serialize for StorageSecretValue<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        match self.0 {
            SecretValue::Inline { value } => {
                map.serialize_entry("mode", "Inline")?;
                map.serialize_entry("value", value.expose_secret())?;
            }
            SecretValue::FilePath { path } => {
                map.serialize_entry("mode", "FilePath")?;
                map.serialize_entry("path", path)?;
            }
        }
        map.end()
    }
}

/// Newtype that serializes `CredentialType` with all secret fields exposed.
/// Use this for database storage only — the default `Serialize` impl redacts secrets.
pub struct StorageCredentialType<'a>(pub &'a CredentialType);

impl Serialize for StorageCredentialType<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            CredentialType::Snmp { version, community } => {
                let mut map = serializer.serialize_map(Some(3))?;
                map.serialize_entry("type", "Snmp")?;
                map.serialize_entry("version", version)?;
                map.serialize_entry("community", &StorageSecretValue(community))?;
                map.end()
            }
            CredentialType::DockerProxy {
                port,
                path,
                ssl_cert,
                ssl_key,
                ssl_chain,
            } => {
                let mut map = serializer.serialize_map(Some(6))?;
                map.serialize_entry("type", "DockerProxy")?;
                map.serialize_entry("port", port)?;
                map.serialize_entry("path", path)?;
                map.serialize_entry("ssl_cert", ssl_cert)?;
                match ssl_key {
                    Some(sv) => map.serialize_entry("ssl_key", &StorageSecretValue(sv))?,
                    None => map.serialize_entry("ssl_key", &None::<()>)?,
                }
                map.serialize_entry("ssl_chain", ssl_chain)?;
                map.end()
            }
        }
    }
}

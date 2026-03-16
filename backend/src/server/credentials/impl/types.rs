use crate::server::shared::{
    entities::EntityDiscriminants,
    types::{
        Color, Icon,
        metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
    },
};
use secrecy::SecretString;
use serde::{Deserialize, Serialize, Serializer};
use strum::VariantNames;
use strum_macros::EnumIter;
use utoipa::ToSchema;

/// Serializer that redacts the secret value
fn redact_secret<S>(_secret: &SecretString, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str("********")
}

fn redact_optional_secret<S>(
    secret: &Option<SecretString>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match secret {
        Some(_) => serializer.serialize_str("********"),
        None => serializer.serialize_none(),
    }
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
        #[serde(serialize_with = "redact_secret")]
        #[schema(value_type = String)]
        community: SecretString,
    },
    /// Docker proxy with SSL certs as local file paths on the daemon host.
    DockerProxyLocal {
        url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ssl_cert_path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ssl_key_path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ssl_chain_path: Option<String>,
    },
    /// Docker proxy with inline SSL cert content for remote hosts.
    DockerProxyRemote {
        url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ssl_cert: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[serde(serialize_with = "redact_optional_secret")]
        #[schema(value_type = Option<String>)]
        ssl_key: Option<SecretString>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ssl_chain: Option<String>,
    },
}

impl PartialEq for CredentialType {
    fn eq(&self, other: &Self) -> bool {
        use secrecy::ExposeSecret;
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
            ) => v1 == v2 && c1.expose_secret() == c2.expose_secret(),
            (
                Self::DockerProxyLocal {
                    url: u1,
                    ssl_cert_path: cp1,
                    ssl_key_path: kp1,
                    ssl_chain_path: chp1,
                },
                Self::DockerProxyLocal {
                    url: u2,
                    ssl_cert_path: cp2,
                    ssl_key_path: kp2,
                    ssl_chain_path: chp2,
                },
            ) => u1 == u2 && cp1 == cp2 && kp1 == kp2 && chp1 == chp2,
            (
                Self::DockerProxyRemote {
                    url: u1,
                    ssl_cert: c1,
                    ssl_key: k1,
                    ssl_chain: ch1,
                },
                Self::DockerProxyRemote {
                    url: u2,
                    ssl_cert: c2,
                    ssl_key: k2,
                    ssl_chain: ch2,
                },
            ) => {
                u1 == u2
                    && c1 == c2
                    && match (k1, k2) {
                        (Some(a), Some(b)) => a.expose_secret() == b.expose_secret(),
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
            Self::DockerProxyLocal { .. } => "DockerProxyLocal",
            Self::DockerProxyRemote { .. } => "DockerProxyRemote",
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
                    options: Some(SnmpVersion::VARIANTS),
                    default_value: Some("V2c"),
                },
                FieldDefinition {
                    id: "community",
                    label: "Community String",
                    field_type: FieldType::String,
                    placeholder: Some("e.g. public"),
                    secret: true,
                    optional: false,
                    help_text: Some("The SNMP community string used to authenticate with devices"),
                    options: None,
                    default_value: None,
                },
            ],
            Self::DockerProxyLocal {
                url: _,
                ssl_cert_path: _,
                ssl_key_path: _,
                ssl_chain_path: _,
            } => vec![
                FieldDefinition {
                    id: "url",
                    label: "Proxy URL",
                    field_type: FieldType::String,
                    placeholder: Some("https://localhost:2376"),
                    secret: false,
                    optional: false,
                    help_text: Some("URL of the Docker API proxy on the daemon host"),
                    options: None,
                    default_value: None,
                },
                FieldDefinition {
                    id: "ssl_cert_path",
                    label: "SSL Certificate Path",
                    field_type: FieldType::String,
                    placeholder: Some("/etc/docker/certs/cert.pem"),
                    secret: false,
                    optional: true,
                    help_text: None,
                    options: None,
                    default_value: None,
                },
                FieldDefinition {
                    id: "ssl_key_path",
                    label: "SSL Key Path",
                    field_type: FieldType::String,
                    placeholder: Some("/etc/docker/certs/key.pem"),
                    secret: false,
                    optional: true,
                    help_text: None,
                    options: None,
                    default_value: None,
                },
                FieldDefinition {
                    id: "ssl_chain_path",
                    label: "SSL Chain Path",
                    field_type: FieldType::String,
                    placeholder: Some("/etc/docker/certs/ca.pem"),
                    secret: false,
                    optional: true,
                    help_text: None,
                    options: None,
                    default_value: None,
                },
            ],
            Self::DockerProxyRemote {
                url: _,
                ssl_cert: _,
                ssl_key: _,
                ssl_chain: _,
            } => vec![
                FieldDefinition {
                    id: "url",
                    label: "Proxy URL",
                    field_type: FieldType::String,
                    placeholder: Some("https://192.168.1.50:2376"),
                    secret: false,
                    optional: false,
                    help_text: Some("URL of the Docker API proxy on the remote host"),
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
                    help_text: Some("PEM-encoded public certificate"),
                    options: None,
                    default_value: None,
                },
                FieldDefinition {
                    id: "ssl_key",
                    label: "SSL Private Key",
                    field_type: FieldType::Text,
                    placeholder: Some("-----BEGIN PRIVATE KEY-----"),
                    secret: true,
                    optional: true,
                    help_text: Some("PEM-encoded private key"),
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
                    help_text: None,
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
    DockerProxyLocal,
    DockerProxyRemote,
}

impl CredentialTypeVariant {
    pub fn to_credential_type(&self) -> CredentialType {
        match self {
            Self::Snmp => CredentialType::Snmp {
                version: SnmpVersion::default(),
                community: SecretString::from(String::new()),
            },
            Self::DockerProxyLocal => CredentialType::DockerProxyLocal {
                url: String::new(),
                ssl_cert_path: None,
                ssl_key_path: None,
                ssl_chain_path: None,
            },
            Self::DockerProxyRemote => CredentialType::DockerProxyRemote {
                url: String::new(),
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

impl TypeMetadataProvider for CredentialType {
    fn name(&self) -> &'static str {
        match self {
            Self::Snmp { .. } => "SNMP",
            Self::DockerProxyLocal { .. } => "Docker Proxy",
            Self::DockerProxyRemote { .. } => "Docker Proxy (Remote)",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::Snmp { .. } => "SNMP community string for querying network devices",
            Self::DockerProxyLocal { .. } => "Docker API proxy with local SSL certificate files",
            Self::DockerProxyRemote { .. } => {
                "Docker API proxy with inline SSL certificates for remote hosts"
            }
        }
    }

    fn metadata(&self) -> serde_json::Value {
        serde_json::json!({ "fields": self.field_definitions() })
    }
}

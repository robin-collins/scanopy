use crate::server::{
    credentials::r#impl::mapping::{
        CredentialQueryPayload, DockerProxyQueryCredential, ResolvableSecret, ResolvableValue,
    },
    ports::r#impl::base::PortType,
    shared::{
        concepts::Concept,
        types::{
            Color, Icon,
            metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
        },
    },
};
use anyhow::Error;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize, Serializer, ser::SerializeMap};
use strum::{Display, EnumDiscriminants, IntoDiscriminant};
use strum_macros::{EnumIter, IntoStaticStr};
use utoipa::ToSchema;
use uuid::Uuid;

pub mod docker_proxy;
pub mod snmp;

/// Category grouping for credential types.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, IntoStaticStr, ToSchema, PartialEq, Eq)]
pub enum CredentialCategory {
    /// Network monitoring protocols (SNMP, NetFlow, sFlow)
    #[strum(serialize = "Network Monitoring")]
    NetworkMonitoring,
    /// Container and virtualization platforms (Docker, vSphere, ESXi)
    #[strum(serialize = "Container & Virtualization")]
    ContainerVirtualization,
}

/// How a credential is scoped to targets.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub enum ScopeModel {
    /// Network default — try on all hosts with matching open ports
    Broadcast,
    /// Assigned to specific hosts only
    PerHost,
}

// Re-export SnmpVersion from snmp submodule
pub use snmp::types::SnmpVersion;

/// Sentinel value used by the `redact_secret` serializer.
/// The frontend also hardcodes this value for show/hide toggle logic.
pub const REDACTED_SECRET_SENTINEL: &str = "********";

/// Serializer that redacts the secret value
fn redact_secret<S>(_secret: &SecretString, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(REDACTED_SECRET_SENTINEL)
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

impl SecretValue {
    /// Returns true if this secret value contains the redacted sentinel.
    pub fn is_redacted_sentinel(&self) -> bool {
        match self {
            SecretValue::Inline { value } => value.expose_secret() == REDACTED_SECRET_SENTINEL,
            SecretValue::FilePath { .. } => false,
        }
    }
}

/// Non-secret value that can be inline content or a file path on daemon host.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(tag = "mode")]
pub enum FileOrInline {
    Inline { value: String },
    FilePath { path: String },
}

fn default_docker_port() -> u16 {
    PortType::DockerTls.number()
}

/// Universal credential type — tagged enum stored as JSONB.
/// Each variant represents a different credential protocol/method.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, EnumDiscriminants, IntoStaticStr)]
#[strum_discriminants(derive(Display, Hash, Serialize, Deserialize, IntoStaticStr))]
#[serde(tag = "type")]
pub enum CredentialType {
    /// SNMPv2c community string for querying network devices
    SnmpV2c { community: SecretValue },
    /// Docker API proxy credentials. Target IP determined from host interfaces at scan time.
    DockerProxy {
        /// Port for the Docker API proxy (default 2376)
        #[serde(default = "default_docker_port")]
        port: u16,
        /// Optional URL path prefix (e.g. "/v1.43")
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        /// PEM-encoded public certificate — inline or file path on daemon host
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ssl_cert: Option<FileOrInline>,
        /// Private key — inline PEM content or file path on daemon host
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ssl_key: Option<SecretValue>,
        /// PEM-encoded CA chain — inline or file path on daemon host
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ssl_chain: Option<FileOrInline>,
    },
}

impl PartialEq for CredentialType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::SnmpV2c { community: c1 }, Self::SnmpV2c { community: c2 }) => match (c1, c2) {
                (SecretValue::Inline { value: a }, SecretValue::Inline { value: b }) => {
                    a.expose_secret() == b.expose_secret()
                }
                (SecretValue::FilePath { path: a }, SecretValue::FilePath { path: b }) => a == b,
                _ => false,
            },
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
    /// Merge redacted sentinel values from the existing credential.
    /// When the API response redacts secrets to "********" and the UI sends that back,
    /// this restores the original secret from the existing record.
    pub fn merge_redacted_secrets(&mut self, existing: &CredentialType) {
        match (self, existing) {
            (
                Self::SnmpV2c { community },
                Self::SnmpV2c {
                    community: existing_community,
                },
            ) => {
                if community.is_redacted_sentinel() {
                    *community = existing_community.clone();
                }
            }
            (
                Self::DockerProxy { ssl_key, .. },
                Self::DockerProxy {
                    ssl_key: existing_key,
                    ..
                },
            ) => {
                if let Some(key) = ssl_key
                    && key.is_redacted_sentinel()
                {
                    *ssl_key = existing_key.clone();
                }
            }
            // Type changed — no merging needed
            _ => {}
        }
    }

    pub fn credential_category(&self) -> CredentialCategory {
        match self {
            Self::SnmpV2c { .. } => CredentialCategory::NetworkMonitoring,
            Self::DockerProxy { .. } => CredentialCategory::ContainerVirtualization,
        }
    }

    pub fn scope_models(&self) -> Vec<ScopeModel> {
        match self {
            Self::SnmpV2c { .. } => vec![ScopeModel::Broadcast, ScopeModel::PerHost],
            Self::DockerProxy { .. } => vec![ScopeModel::PerHost],
        }
    }

    /// Get the inline string value for a field by ID.
    /// Returns None for FilePath mode, None fields, or redacted sentinels.
    fn get_inline_value(&self, field_id: &str) -> Option<String> {
        match self {
            Self::SnmpV2c { community } => match field_id {
                "community" => match community {
                    SecretValue::Inline { value } => {
                        let v = value.expose_secret().to_string();
                        if v == REDACTED_SECRET_SENTINEL {
                            None
                        } else {
                            Some(v)
                        }
                    }
                    SecretValue::FilePath { .. } => None,
                },
                _ => None,
            },
            Self::DockerProxy {
                ssl_cert,
                ssl_key,
                ssl_chain,
                ..
            } => match field_id {
                "ssl_cert" => match ssl_cert.as_ref()? {
                    FileOrInline::Inline { value } => Some(value.clone()),
                    FileOrInline::FilePath { .. } => None,
                },
                "ssl_key" => match ssl_key.as_ref()? {
                    SecretValue::Inline { value } => {
                        let v = value.expose_secret().to_string();
                        if v == REDACTED_SECRET_SENTINEL {
                            None
                        } else {
                            Some(v)
                        }
                    }
                    SecretValue::FilePath { .. } => None,
                },
                "ssl_chain" => match ssl_chain.as_ref()? {
                    FileOrInline::Inline { value } => Some(value.clone()),
                    FileOrInline::FilePath { .. } => None,
                },
                _ => None,
            },
        }
    }

    /// Validate inline field values using field_definitions() metadata.
    /// Skips FilePath values (validated on daemon after read), redacted sentinels,
    /// and empty optionals.
    pub fn validate(&self) -> Result<(), Error> {
        for field in self.field_definitions() {
            if let Some(fmt) = &field.inline_format
                && let Some(value) = self.get_inline_value(field.id)
            {
                fmt.validate(&value, field.label)?;
            }
        }
        Ok(())
    }

    /// Ports that must be open on a host for this credential to be applicable during discovery.
    /// Empty vec means the credential applies regardless of open ports.
    /// When multiple ports are returned, the credential applies if *any* of them are open.
    pub fn required_ports(&self) -> Vec<PortType> {
        match self {
            Self::SnmpV2c { .. } => vec![PortType::Snmp, PortType::SnmpAlt],
            Self::DockerProxy { port, .. } => vec![PortType::new_tcp(*port)],
        }
    }

    /// Human-readable port/protocol description derived from required_ports().
    /// Uses PortType's Display impl which formats as "number/protocol" (e.g. "161/udp").
    pub fn port_description(&self) -> String {
        self.required_ports()
            .first()
            .map(|p| p.to_string())
            .unwrap_or_default()
    }

    /// If this credential type allows a user-configured port, return the field ID.
    /// Used by the frontend to read the actual port from credential data.
    pub fn custom_port_field(&self) -> Option<&'static str> {
        match self {
            Self::SnmpV2c { .. } => None,
            Self::DockerProxy { .. } => Some("port"),
        }
    }

    /// Convert to wire format payload for daemon transmission.
    /// No wildcard match — compiler forces update when new variants added.
    pub fn to_query_payload(&self) -> CredentialQueryPayload {
        match self {
            CredentialType::SnmpV2c { community } => CredentialQueryPayload::Snmp(
                crate::server::credentials::r#impl::mapping::SnmpQueryCredential {
                    version: SnmpVersion::V2c,
                    community: match community {
                        SecretValue::Inline { value } => ResolvableSecret::Value {
                            value: value.expose_secret().to_string(),
                        },
                        SecretValue::FilePath { path } => {
                            ResolvableSecret::FilePath { path: path.clone() }
                        }
                    },
                },
            ),
            CredentialType::DockerProxy {
                port,
                path,
                ssl_cert,
                ssl_key,
                ssl_chain,
            } => CredentialQueryPayload::DockerProxy(DockerProxyQueryCredential {
                port: *port,
                path: path.clone(),
                ssl_cert: ssl_cert.as_ref().map(|f| match f {
                    FileOrInline::Inline { value } => ResolvableValue::Value {
                        value: value.clone(),
                    },
                    FileOrInline::FilePath { path } => {
                        ResolvableValue::FilePath { path: path.clone() }
                    }
                }),
                ssl_key: ssl_key.as_ref().map(|s| match s {
                    SecretValue::Inline { value } => ResolvableSecret::Value {
                        value: value.expose_secret().to_string(),
                    },
                    SecretValue::FilePath { path } => {
                        ResolvableSecret::FilePath { path: path.clone() }
                    }
                }),
                ssl_chain: ssl_chain.as_ref().map(|f| match f {
                    FileOrInline::Inline { value } => ResolvableValue::Value {
                        value: value.clone(),
                    },
                    FileOrInline::FilePath { path } => {
                        ResolvableValue::FilePath { path: path.clone() }
                    }
                }),
            }),
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
    /// For SecretPathOrInline fields: what format the inline value should be
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_format: Option<InlineFormat>,
    /// Optional group name for visually grouping fields in the UI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<&'static str>,
}

/// Format hint for inline values in PathOrInline and SecretPathOrInline fields.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum InlineFormat {
    /// Plain text (e.g. SNMP community string, API key)
    Plain,
    /// PEM-encoded private key
    PemPrivateKey,
    /// PEM-encoded certificate (public, non-secret)
    PemCertificate,
}

/// PEM block tag — the label between `-----BEGIN` and `-----`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PemTag {
    Certificate,
    PrivateKey,
    RsaPrivateKey,
    EcPrivateKey,
}

impl PemTag {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Certificate => "CERTIFICATE",
            Self::PrivateKey => "PRIVATE KEY",
            Self::RsaPrivateKey => "RSA PRIVATE KEY",
            Self::EcPrivateKey => "EC PRIVATE KEY",
        }
    }
}

impl InlineFormat {
    /// PEM tags accepted by this format, or empty for non-PEM formats.
    pub fn allowed_pem_tags(&self) -> &'static [PemTag] {
        match self {
            Self::Plain => &[],
            Self::PemCertificate => &[PemTag::Certificate],
            Self::PemPrivateKey => &[
                PemTag::PrivateKey,
                PemTag::RsaPrivateKey,
                PemTag::EcPrivateKey,
            ],
        }
    }

    /// Validate a resolved value matches the expected format.
    /// Returns Ok(()) for Plain format (no validation needed).
    pub fn validate(&self, value: &str, field_name: &str) -> Result<(), Error> {
        let tags = self.allowed_pem_tags();
        if tags.is_empty() {
            return Ok(());
        }
        validate_pem(value, field_name, tags)
    }
}

/// Parse PEM and verify at least one entry has a tag in `allowed_tags`.
fn validate_pem(value: &str, field_name: &str, allowed_tags: &[PemTag]) -> Result<(), Error> {
    use crate::server::shared::types::api::ValidationError;

    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    let entries = pem::parse_many(trimmed)
        .map_err(|e| ValidationError::new(format!("{} is not valid PEM: {}", field_name, e)))?;
    if entries.is_empty() {
        crate::bail_validation!("{} contains no PEM data", field_name);
    }
    if !entries
        .iter()
        .any(|p| allowed_tags.iter().any(|t| t.as_str() == p.tag()))
    {
        let expected = allowed_tags
            .iter()
            .map(|t| t.as_str())
            .collect::<Vec<_>>()
            .join(" or ");
        crate::bail_validation!(
            "{} must contain a {} PEM block, found: {}",
            field_name,
            expected,
            entries
                .iter()
                .map(|p| p.tag().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    String,
    Text,
    Select,
    SecretPathOrInline,
    PathOrInline,
}

impl CredentialType {
    /// Returns field definitions for this credential type.
    /// Uses exhaustive destructuring for compile-time enforcement:
    /// adding a field to the enum variant without updating this method causes a compile error.
    pub fn field_definitions(&self) -> Vec<FieldDefinition> {
        match self {
            Self::SnmpV2c { community: _ } => vec![FieldDefinition {
                id: "community",
                label: "Community String",
                field_type: FieldType::SecretPathOrInline,
                placeholder: Some("custom-community-string"),
                secret: true,
                optional: false,
                help_text: Some(
                    "Custom SNMP community string. The default 'public' community is always tried automatically during scans.",
                ),
                options: None,
                default_value: None,
                inline_format: Some(InlineFormat::Plain),
                group: None,
            }],
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
                    inline_format: None,
                    group: Some("Connection"),
                },
                FieldDefinition {
                    id: "path",
                    label: "URL Path Prefix",
                    field_type: FieldType::String,
                    placeholder: Some("/"),
                    secret: false,
                    optional: true,
                    help_text: Some("Optional URL path prefix appended after the port"),
                    options: None,
                    default_value: None,
                    inline_format: None,
                    group: Some("Connection"),
                },
                FieldDefinition {
                    id: "ssl_cert",
                    label: "SSL Certificate",
                    field_type: FieldType::PathOrInline,
                    placeholder: Some("-----BEGIN CERTIFICATE-----"),
                    secret: false,
                    optional: true,
                    help_text: Some("PEM-encoded client certificate"),
                    options: None,
                    default_value: None,
                    inline_format: Some(InlineFormat::PemCertificate),
                    group: Some("TLS"),
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
                    inline_format: Some(InlineFormat::PemPrivateKey),
                    group: Some("TLS"),
                },
                FieldDefinition {
                    id: "ssl_chain",
                    label: "SSL CA Chain",
                    field_type: FieldType::PathOrInline,
                    placeholder: Some("-----BEGIN CERTIFICATE-----"),
                    secret: false,
                    optional: true,
                    help_text: Some("PEM-encoded CA certificate chain"),
                    options: None,
                    default_value: None,
                    inline_format: Some(InlineFormat::PemCertificate),
                    group: Some("TLS"),
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
    SnmpV2c,
    DockerProxy,
}

impl CredentialTypeVariant {
    pub fn to_credential_type(&self) -> CredentialType {
        match self {
            Self::SnmpV2c => CredentialType::SnmpV2c {
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
        self.discriminant().into()
    }
}

impl EntityMetadataProvider for CredentialType {
    fn color(&self) -> Color {
        match self {
            Self::SnmpV2c { .. } => Concept::SNMP.color(),
            Self::DockerProxy { .. } => Concept::Virtualization.color(),
        }
    }
    fn icon(&self) -> Icon {
        match self {
            Self::SnmpV2c { .. } => Concept::SNMP.icon(),
            Self::DockerProxy { .. } => Concept::Virtualization.icon(),
        }
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
            Self::SnmpV2c { .. } => "SNMP v2c",
            Self::DockerProxy { .. } => "Docker Proxy",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::SnmpV2c { .. } => "SNMPv2c community string for querying network devices",
            Self::DockerProxy { .. } => {
                "Docker API proxy credentials. TLS is optional."
            }
        }
    }

    fn category(&self) -> &'static str {
        self.credential_category().into()
    }

    fn metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "fields": self.field_definitions(),
            "port_description": self.port_description(),
            "custom_port_field": self.custom_port_field(),
            "scope_models": self.scope_models(),
        })
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
            CredentialType::SnmpV2c { community } => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("type", "SnmpV2c")?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn snmp_cred(community: &str) -> CredentialType {
        CredentialType::SnmpV2c {
            community: SecretValue::Inline {
                value: SecretString::from(community.to_string()),
            },
        }
    }

    fn docker_cred(ssl_key: Option<&str>) -> CredentialType {
        CredentialType::DockerProxy {
            port: 2376,
            path: None,
            ssl_cert: None,
            ssl_key: ssl_key.map(|k| SecretValue::Inline {
                value: SecretString::from(k.to_string()),
            }),
            ssl_chain: None,
        }
    }

    #[test]
    fn merge_redacted_secrets_preserves_original_when_sentinel_sent() {
        let existing = snmp_cred("my-secret-community");
        let mut updated = snmp_cred(REDACTED_SECRET_SENTINEL);
        updated.merge_redacted_secrets(&existing);

        if let CredentialType::SnmpV2c { community } = &updated {
            if let SecretValue::Inline { value } = community {
                assert_eq!(value.expose_secret(), "my-secret-community");
            } else {
                panic!("Expected Inline secret");
            }
        } else {
            panic!("Expected Snmp variant");
        }
    }

    #[test]
    fn merge_redacted_secrets_allows_actual_value_changes() {
        let existing = snmp_cred("old-community");
        let mut updated = snmp_cred("new-community");
        updated.merge_redacted_secrets(&existing);

        if let CredentialType::SnmpV2c { community } = &updated {
            if let SecretValue::Inline { value } = community {
                assert_eq!(value.expose_secret(), "new-community");
            } else {
                panic!("Expected Inline secret");
            }
        } else {
            panic!("Expected Snmp variant");
        }
    }

    #[test]
    fn merge_redacted_secrets_handles_type_mismatch() {
        let existing = snmp_cred("secret");
        let mut updated = docker_cred(Some("new-key"));
        // Should be a no-op — types don't match
        updated.merge_redacted_secrets(&existing);

        if let CredentialType::DockerProxy { ssl_key, .. } = &updated {
            if let Some(SecretValue::Inline { value }) = ssl_key {
                assert_eq!(value.expose_secret(), "new-key");
            } else {
                panic!("Expected Some(Inline)");
            }
        } else {
            panic!("Expected DockerProxy variant");
        }
    }

    #[test]
    fn merge_redacted_secrets_handles_docker_ssl_key() {
        let existing = docker_cred(Some("original-key"));
        let mut updated = docker_cred(Some(REDACTED_SECRET_SENTINEL));
        updated.merge_redacted_secrets(&existing);

        if let CredentialType::DockerProxy { ssl_key, .. } = &updated {
            if let Some(SecretValue::Inline { value }) = ssl_key {
                assert_eq!(value.expose_secret(), "original-key");
            } else {
                panic!("Expected Some(Inline)");
            }
        } else {
            panic!("Expected DockerProxy variant");
        }
    }

    #[test]
    fn is_redacted_sentinel_detects_sentinel() {
        let sentinel = SecretValue::Inline {
            value: SecretString::from(REDACTED_SECRET_SENTINEL.to_string()),
        };
        assert!(sentinel.is_redacted_sentinel());

        let real = SecretValue::Inline {
            value: SecretString::from("real-secret".to_string()),
        };
        assert!(!real.is_redacted_sentinel());

        let filepath = SecretValue::FilePath {
            path: REDACTED_SECRET_SENTINEL.to_string(),
        };
        assert!(!filepath.is_redacted_sentinel());
    }
}

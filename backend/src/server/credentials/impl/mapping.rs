//! Generic credential mapping for discovery dispatch.
//!
//! The mapping types define how credentials are resolved per-IP during discovery.
//! `CredentialMapping<T>` is generic over the query credential type.

use crate::server::ports::r#impl::base::PortType;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::path::Path;
use utoipa::ToSchema;

// Re-export type-specific types so external imports don't break
pub use super::types::docker_proxy::types::DockerProxyQueryCredential;
pub use super::types::snmp::types::{
    LegacySnmpCredentialMapping, SnmpCredentialMapping, SnmpCredentialMappingExposed,
    SnmpIpOverrideExposed, SnmpQueryCredential, SnmpQueryCredentialExposed, SnmpVersion,
};

// ============================================================================
// Generic Credential Mapping
// ============================================================================

/// Generic credential mapping: a default credential for the network
/// plus per-IP overrides for specific hosts.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct CredentialMapping<T> {
    #[serde(default)]
    pub default_credential: Option<T>,
    #[serde(default)]
    pub ip_overrides: Vec<IpOverride<T>>,
    /// Ports that must be open for this credential type to be applicable.
    #[serde(default)]
    pub required_ports: Vec<PortType>,
}

/// IP-specific credential override
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct IpOverride<T> {
    pub ip: IpAddr,
    pub credential: T,
}

impl<T> CredentialMapping<T> {
    /// Check if any credentials are configured
    pub fn is_enabled(&self) -> bool {
        self.default_credential.is_some() || !self.ip_overrides.is_empty()
    }

    /// Get credential for a specific IP, falling back to default
    pub fn get_credential_for_ip(&self, ip: &IpAddr) -> Option<&T> {
        self.ip_overrides
            .iter()
            .find(|o| &o.ip == ip)
            .map(|o| &o.credential)
            .or(self.default_credential.as_ref())
    }
}

// ============================================================================
// Generic Credential Query Types (wire format for unified discovery)
// ============================================================================

/// Credential payload sent to daemon with secrets exposed.
/// Each variant corresponds to a CredentialType variant.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(tag = "type")]
pub enum CredentialQueryPayload {
    Snmp(SnmpQueryCredential),
    DockerProxy(DockerProxyQueryCredential),
}

impl Default for CredentialQueryPayload {
    fn default() -> Self {
        Self::Snmp(SnmpQueryCredential::default())
    }
}

impl CredentialQueryPayload {
    pub fn discovery_label(&self) -> &'static str {
        match self {
            Self::Snmp(_) => "SNMP queries",
            Self::DockerProxy(_) => "Docker proxy connection",
        }
    }

    /// Resolve all FilePath fields to Value by reading from disk.
    /// Value fields pass through unchanged.
    pub fn resolve_file_paths(&self) -> Result<Self, anyhow::Error> {
        let label = self.discovery_label();
        match self {
            Self::Snmp(snmp) => Ok(Self::Snmp(SnmpQueryCredential {
                version: snmp.version,
                community: snmp.community.resolve_to_value("community", label)?,
            })),
            Self::DockerProxy(d) => Ok(Self::DockerProxy(DockerProxyQueryCredential {
                port: d.port,
                path: d.path.clone(),
                ssl_cert: d
                    .ssl_cert
                    .as_ref()
                    .map(|v| v.resolve_to_value("ssl_cert", label))
                    .transpose()?,
                ssl_key: d
                    .ssl_key
                    .as_ref()
                    .map(|v| v.resolve_to_value("ssl_key", label))
                    .transpose()?,
                ssl_chain: d
                    .ssl_chain
                    .as_ref()
                    .map(|v| v.resolve_to_value("ssl_chain", label))
                    .transpose()?,
            })),
        }
    }

    pub fn banner_lines(&self) -> Vec<BannerField> {
        match self {
            Self::Snmp(snmp) => snmp.banner_lines(),
            Self::DockerProxy(docker) => docker.banner_lines(),
        }
    }
}

/// Non-secret value — inline or file path. Daemon can log freely.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(tag = "mode")]
pub enum ResolvableValue {
    Value { value: String },
    FilePath { path: String },
}

/// Secret value — inline or file path. Daemon wraps resolved value in Secret<String>.
/// Never logged in plaintext.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, ToSchema)]
#[serde(tag = "mode")]
pub enum ResolvableSecret {
    Value { value: String },
    FilePath { path: String },
}

impl ResolvableValue {
    /// Resolve to a string value. FilePath variant reads from disk.
    pub fn resolve(&self, field_name: &str, label: &str) -> Result<String, anyhow::Error> {
        match self {
            Self::Value { value } => Ok(value.clone()),
            Self::FilePath { path } => {
                tracing::info!("Read {} from {} for {}", field_name, path, label);
                std::fs::read_to_string(path).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to read {} from {} for {}: {}",
                        field_name,
                        path,
                        label,
                        e
                    )
                })
            }
        }
    }

    /// Read FilePath from disk and return Value. Value variants pass through.
    pub fn resolve_to_value(&self, field_name: &str, label: &str) -> Result<Self, anyhow::Error> {
        match self {
            Self::Value { .. } => Ok(self.clone()),
            Self::FilePath { path } => {
                tracing::info!("Read {} from {} for {}", field_name, path, label);
                let contents = std::fs::read_to_string(path).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to read {} from {} for {}: {}",
                        field_name,
                        path,
                        label,
                        e
                    )
                })?;
                Ok(Self::Value { value: contents })
            }
        }
    }
}

impl ResolvableSecret {
    /// Resolve to a Secret<String>. FilePath variant reads from disk.
    pub fn resolve(
        &self,
        field_name: &str,
        label: &str,
    ) -> Result<redact::Secret<String>, anyhow::Error> {
        match self {
            Self::Value { value } => Ok(redact::Secret::from(value.clone())),
            Self::FilePath { path } => {
                tracing::info!("Read {} (********) from {} for {}", field_name, path, label);
                let contents = std::fs::read_to_string(path).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to read {} from {} for {}: {}",
                        field_name,
                        path,
                        label,
                        e
                    )
                })?;
                Ok(redact::Secret::from(contents))
            }
        }
    }

    /// Read FilePath from disk and return Value. Value variants pass through.
    pub fn resolve_to_value(&self, field_name: &str, label: &str) -> Result<Self, anyhow::Error> {
        match self {
            Self::Value { .. } => Ok(self.clone()),
            Self::FilePath { path } => {
                tracing::info!("Read {} (********) from {} for {}", field_name, path, label);
                let contents = std::fs::read_to_string(path).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to read {} from {} for {}: {}",
                        field_name,
                        path,
                        label,
                        e
                    )
                })?;
                Ok(Self::Value { value: contents })
            }
        }
    }
}

// ============================================================================
// Banner display types for credential logging
// ============================================================================

/// One line in the credential banner.
pub struct BannerField {
    pub label: &'static str,
    pub value: BannerFieldValue,
}

pub enum BannerFieldValue {
    /// Non-secret inline value — show directly (e.g., port "2376", version "v2c")
    Plain(String),
    /// Inline secret — show "********"
    RedactedInline,
    /// File path that exists — show "successfully read from /path"
    FileOk(String),
    /// File path that doesn't exist — show "failed to read from /path"
    FileFailed(String),
}

impl BannerFieldValue {
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::FileFailed(_))
    }
}

impl std::fmt::Display for BannerFieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Plain(v) => write!(f, "{}", v),
            Self::RedactedInline => write!(f, "********"),
            Self::FileOk(path) => write!(f, "successfully read from {}", path),
            Self::FileFailed(path) => write!(f, "failed to read from {}", path),
        }
    }
}

impl ResolvableValue {
    pub fn banner_value(&self) -> BannerFieldValue {
        match self {
            Self::Value { value } => BannerFieldValue::Plain(value.clone()),
            Self::FilePath { path } => {
                if Path::new(path).exists() {
                    BannerFieldValue::FileOk(path.clone())
                } else {
                    BannerFieldValue::FileFailed(path.clone())
                }
            }
        }
    }
}

impl ResolvableSecret {
    pub fn banner_value(&self) -> BannerFieldValue {
        match self {
            Self::Value { .. } => BannerFieldValue::RedactedInline,
            Self::FilePath { path } => {
                if Path::new(path).exists() {
                    BannerFieldValue::FileOk(path.clone())
                } else {
                    BannerFieldValue::FileFailed(path.clone())
                }
            }
        }
    }
}

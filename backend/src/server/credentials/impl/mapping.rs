//! Generic credential mapping for discovery dispatch.
//!
//! The mapping types define how credentials are resolved per-IP during discovery.
//! `CredentialMapping<T>` is generic over the query credential type.

use crate::server::{credentials::r#impl::types::SnmpVersion, ports::r#impl::base::PortType};
use redact::Secret;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::path::Path;
use utoipa::ToSchema;

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
// SNMP Query Types (wire format — must match daemon expectations)
// ============================================================================

/// Minimal SNMP credential for daemon queries (version + community only)
#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Hash, ToSchema)]
pub struct SnmpQueryCredential {
    #[serde(default)]
    pub version: SnmpVersion,
    pub community: ResolvableSecret,
}

impl Default for SnmpQueryCredential {
    fn default() -> Self {
        Self {
            version: SnmpVersion::default(),
            community: ResolvableSecret::Value {
                value: String::new(),
            },
        }
    }
}

impl std::fmt::Debug for SnmpQueryCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SnmpQueryCredential")
            .field("version", &self.version)
            .field("community", &"********")
            .finish()
    }
}

impl SnmpQueryCredential {
    pub fn public_default() -> Self {
        Self {
            version: SnmpVersion::default(),
            community: ResolvableSecret::Value {
                value: "public".to_string(),
            },
        }
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
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct DockerProxyQueryCredential {
    pub port: u16,
    pub path: Option<String>,
    pub ssl_cert: Option<ResolvableValue>,
    pub ssl_key: Option<ResolvableSecret>,
    pub ssl_chain: Option<ResolvableValue>,
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
    pub fn resolve(&self, field_name: &str, label: &str) -> Result<Secret<String>, anyhow::Error> {
        match self {
            Self::Value { value } => Ok(Secret::from(value.clone())),
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
                Ok(Secret::from(contents))
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

impl CredentialQueryPayload {
    pub fn banner_lines(&self) -> Vec<BannerField> {
        match self {
            Self::Snmp(snmp) => vec![
                BannerField {
                    label: "Community",
                    value: snmp.community.banner_value(),
                },
                BannerField {
                    label: "Version",
                    value: BannerFieldValue::Plain(snmp.version.to_string()),
                },
            ],
            Self::DockerProxy(docker) => {
                let mut lines = vec![BannerField {
                    label: "Port",
                    value: BannerFieldValue::Plain(docker.port.to_string()),
                }];
                if let Some(ref path) = docker.path {
                    lines.push(BannerField {
                        label: "Path",
                        value: BannerFieldValue::Plain(path.clone()),
                    });
                }
                if let Some(ref cert) = docker.ssl_cert {
                    lines.push(BannerField {
                        label: "SSL cert",
                        value: cert.banner_value(),
                    });
                }
                if let Some(ref key) = docker.ssl_key {
                    lines.push(BannerField {
                        label: "SSL key",
                        value: key.banner_value(),
                    });
                }
                if let Some(ref chain) = docker.ssl_chain {
                    lines.push(BannerField {
                        label: "SSL chain",
                        value: chain.banner_value(),
                    });
                }
                lines
            }
        }
    }
}

/// SNMP credential mapping type alias
pub type SnmpCredentialMapping = CredentialMapping<SnmpQueryCredential>;

/// SNMP-specific resolution: IP override → network default → "public" fallback.
/// Deduplicates by community string.
impl SnmpCredentialMapping {
    pub fn get_credentials_by_specificity(&self, ip: &IpAddr) -> Vec<SnmpQueryCredential> {
        let mut credentials = Vec::new();

        // 1. IP-specific override (most specific)
        if let Some(override_cred) = self.ip_overrides.iter().find(|o| &o.ip == ip) {
            credentials.push(override_cred.credential.clone());
        }

        // 2. Network default
        if let Some(ref default) = self.default_credential
            && !credentials.iter().any(|c| c.community == default.community)
        {
            credentials.push(default.clone());
        }

        // 3. "public" fallback (least specific)
        let public_default = SnmpQueryCredential::public_default();
        if !credentials
            .iter()
            .any(|c| c.community == public_default.community)
        {
            credentials.push(public_default);
        }

        credentials
    }
}

// ============================================================================
// Exposed types for daemon serialization (plaintext secrets)
// ============================================================================

#[derive(Serialize)]
pub struct SnmpQueryCredentialExposed {
    pub version: SnmpVersion,
    pub community: String,
}

impl From<&SnmpQueryCredential> for SnmpQueryCredentialExposed {
    fn from(cred: &SnmpQueryCredential) -> Self {
        Self {
            version: cred.version,
            community: match &cred.community {
                ResolvableSecret::Value { value } => value.clone(),
                ResolvableSecret::FilePath { .. } => String::new(),
            },
        }
    }
}

#[derive(Serialize)]
pub struct SnmpIpOverrideExposed {
    pub ip: IpAddr,
    pub credential: SnmpQueryCredentialExposed,
}

impl From<&IpOverride<SnmpQueryCredential>> for SnmpIpOverrideExposed {
    fn from(o: &IpOverride<SnmpQueryCredential>) -> Self {
        Self {
            ip: o.ip,
            credential: SnmpQueryCredentialExposed::from(&o.credential),
        }
    }
}

#[derive(Serialize)]
pub struct SnmpCredentialMappingExposed {
    pub default_credential: Option<SnmpQueryCredentialExposed>,
    pub ip_overrides: Vec<SnmpIpOverrideExposed>,
    pub required_ports: Vec<PortType>,
}

impl From<&SnmpCredentialMapping> for SnmpCredentialMappingExposed {
    fn from(mapping: &SnmpCredentialMapping) -> Self {
        Self {
            default_credential: mapping.default_credential.as_ref().map(Into::into),
            ip_overrides: mapping.ip_overrides.iter().map(Into::into).collect(),
            required_ports: mapping.required_ports.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    fn cred(community: &str) -> SnmpQueryCredential {
        SnmpQueryCredential {
            version: SnmpVersion::V2c,
            community: ResolvableSecret::Value {
                value: community.to_string(),
            },
        }
    }

    fn community_value(cred: &SnmpQueryCredential) -> &str {
        match &cred.community {
            ResolvableSecret::Value { value } => value,
            ResolvableSecret::FilePath { path } => path,
        }
    }

    #[test]
    fn exposed_serialization_contains_plaintext() {
        let original = SnmpCredentialMapping {
            default_credential: Some(cred("my-secret")),
            ip_overrides: vec![],
            required_ports: vec![],
        };

        let exposed = SnmpCredentialMappingExposed::from(&original);
        let json = serde_json::to_string(&exposed).unwrap();
        assert!(json.contains("my-secret"));
    }

    #[test]
    fn resolvable_secret_roundtrip() {
        let original = cred("my-secret");
        let json = serde_json::to_string(&original).unwrap();
        let roundtripped: SnmpQueryCredential = serde_json::from_str(&json).unwrap();
        assert_eq!(community_value(&roundtripped), "my-secret");
    }

    #[test]
    fn get_credentials_by_specificity_ordering() {
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        let other_ip: IpAddr = "10.0.0.2".parse().unwrap();

        let mapping = SnmpCredentialMapping {
            default_credential: Some(cred("default-community")),
            ip_overrides: vec![IpOverride {
                ip,
                credential: cred("override-community"),
            }],
            required_ports: vec![],
        };

        // IP with override: override first, then default, then public
        let creds = mapping.get_credentials_by_specificity(&ip);
        assert_eq!(creds.len(), 3);
        assert_eq!(community_value(&creds[0]), "override-community");
        assert_eq!(community_value(&creds[1]), "default-community");
        assert_eq!(community_value(&creds[2]), "public");

        // IP without override: default, then public
        let creds = mapping.get_credentials_by_specificity(&other_ip);
        assert_eq!(creds.len(), 2);
        assert_eq!(community_value(&creds[0]), "default-community");
        assert_eq!(community_value(&creds[1]), "public");
    }

    #[test]
    fn get_credentials_by_specificity_deduplicates() {
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        // Override and default are both "public" — should not duplicate
        let mapping = SnmpCredentialMapping {
            default_credential: Some(cred("public")),
            ip_overrides: vec![IpOverride {
                ip,
                credential: cred("public"),
            }],
            required_ports: vec![],
        };

        let creds = mapping.get_credentials_by_specificity(&ip);
        assert_eq!(creds.len(), 1);
        assert_eq!(community_value(&creds[0]), "public");
    }

    #[test]
    fn banner_lines_snmp() {
        let payload = CredentialQueryPayload::Snmp(cred("my-community"));
        let lines = payload.banner_lines();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].label, "Community");
        assert!(matches!(lines[0].value, BannerFieldValue::RedactedInline));
        assert_eq!(lines[1].label, "Version");
        assert!(matches!(&lines[1].value, BannerFieldValue::Plain(v) if v == "V2c"));
    }

    #[test]
    fn banner_lines_docker_proxy() {
        let payload = CredentialQueryPayload::DockerProxy(DockerProxyQueryCredential {
            port: 2376,
            path: Some("/v1.44".to_string()),
            ssl_cert: Some(ResolvableValue::Value {
                value: "cert-content".to_string(),
            }),
            ssl_key: Some(ResolvableSecret::FilePath {
                path: "/nonexistent/key.pem".to_string(),
            }),
            ssl_chain: None,
        });
        let lines = payload.banner_lines();
        assert_eq!(lines.len(), 4); // port, path, ssl_cert, ssl_key
        assert_eq!(lines[0].label, "Port");
        assert!(matches!(&lines[0].value, BannerFieldValue::Plain(v) if v == "2376"));
        assert_eq!(lines[1].label, "Path");
        assert_eq!(lines[2].label, "SSL cert");
        assert!(matches!(&lines[2].value, BannerFieldValue::Plain(v) if v == "cert-content"));
        assert_eq!(lines[3].label, "SSL key");
        assert!(lines[3].value.is_failed());
    }
}

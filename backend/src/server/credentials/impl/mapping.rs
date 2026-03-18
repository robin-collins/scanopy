//! Generic credential mapping for discovery dispatch.
//!
//! The mapping types define how credentials are resolved per-IP during discovery.
//! `CredentialMapping<T>` is generic over the query credential type.

use crate::server::{credentials::r#impl::types::SnmpVersion, ports::r#impl::base::PortType};
use redact::Secret;
use serde::{Deserialize, Serialize, Serializer};
use std::net::IpAddr;
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

/// Serializer that redacts a Secret<String> to "********"
fn redact_secret_string<S: Serializer>(
    _secret: &Secret<String>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str("********")
}

/// Minimal SNMP credential for daemon queries (version + community only)
#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Hash, Default, ToSchema)]
pub struct SnmpQueryCredential {
    #[serde(default)]
    pub version: SnmpVersion,
    pub community: ResolvableSecret,
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
            community: ResolvableSecret::Inline {
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
    Inline { value: String },
    FilePath { path: String },
}

/// Secret value — inline or file path. Daemon wraps resolved value in Secret<String>.
/// Never logged in plaintext.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, ToSchema)]
#[serde(tag = "mode")]
pub enum ResolvableSecret {
    Inline { value: String },
    FilePath { path: String },
}

impl Default for ResolvableSecret {
    fn default() -> Self {
        Self::Inline {
            value: String::new(),
        }
    }
}

impl ResolvableValue {
    /// Resolve to a string value. FilePath variant reads from disk.
    pub fn resolve(&self, field_name: &str, label: &str) -> Result<String, anyhow::Error> {
        match self {
            Self::Inline { value } => Ok(value.clone()),
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
}

impl ResolvableSecret {
    /// Returns the inline value if this is an `Inline` variant, `None` for `FilePath`.
    pub fn inline_value(&self) -> Option<&str> {
        match self {
            Self::Inline { value } => Some(value),
            Self::FilePath { .. } => None,
        }
    }

    /// Resolve to a Secret<String>. FilePath variant reads from disk.
    pub fn resolve(&self, field_name: &str, label: &str) -> Result<Secret<String>, anyhow::Error> {
        match self {
            Self::Inline { value } => Ok(Secret::from(value.clone())),
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
            && !credentials.iter().any(|c| {
                c.community.inline_value().is_some()
                    && c.community.inline_value() == default.community.inline_value()
            })
        {
            credentials.push(default.clone());
        }

        // 3. "public" fallback (least specific)
        if !credentials
            .iter()
            .any(|c| c.community.inline_value() == Some("public"))
        {
            credentials.push(SnmpQueryCredential::public_default());
        }

        credentials
    }
}

// ============================================================================
// Legacy SNMP types for DiscoveryType::Network wire compat
// ============================================================================

/// Legacy SNMP credential format for DiscoveryType::Network wire compat.
/// community is Secret<String> (plain string in JSON) — legacy daemons expect this.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, Default)]
pub struct LegacySnmpQueryCredential {
    #[serde(default)]
    pub version: SnmpVersion,
    #[serde(serialize_with = "redact_secret_string")]
    pub community: Secret<String>,
}

impl TryFrom<&SnmpQueryCredential> for LegacySnmpQueryCredential {
    type Error = ();
    fn try_from(cred: &SnmpQueryCredential) -> Result<Self, ()> {
        match &cred.community {
            ResolvableSecret::Inline { value } => Ok(Self {
                version: cred.version,
                community: Secret::from(value.clone()),
            }),
            ResolvableSecret::FilePath { .. } => Err(()),
        }
    }
}

/// Legacy SNMP credential mapping — uses plain-string community for wire compat.
pub type LegacySnmpCredentialMapping = CredentialMapping<LegacySnmpQueryCredential>;

impl From<&SnmpCredentialMapping> for LegacySnmpCredentialMapping {
    fn from(mapping: &SnmpCredentialMapping) -> Self {
        Self {
            default_credential: mapping
                .default_credential
                .as_ref()
                .and_then(|c| c.try_into().ok()),
            ip_overrides: mapping
                .ip_overrides
                .iter()
                .filter_map(|o| {
                    LegacySnmpQueryCredential::try_from(&o.credential)
                        .ok()
                        .map(|c| IpOverride {
                            ip: o.ip,
                            credential: c,
                        })
                })
                .collect(),
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
            community: ResolvableSecret::Inline {
                value: community.to_string(),
            },
        }
    }

    #[test]
    fn legacy_conversion_preserves_inline_community() {
        let original = SnmpCredentialMapping {
            default_credential: Some(cred("my-secret")),
            ip_overrides: vec![],
            required_ports: vec![],
        };

        let legacy = LegacySnmpCredentialMapping::from(&original);
        assert_eq!(
            legacy
                .default_credential
                .as_ref()
                .unwrap()
                .community
                .expose_secret(),
            "my-secret"
        );
    }

    #[test]
    fn legacy_conversion_skips_filepath() {
        let mapping = SnmpCredentialMapping {
            default_credential: Some(SnmpQueryCredential {
                version: SnmpVersion::V2c,
                community: ResolvableSecret::FilePath {
                    path: "/etc/snmp/community".to_string(),
                },
            }),
            ip_overrides: vec![IpOverride {
                ip: "10.0.0.1".parse().unwrap(),
                credential: cred("inline-community"),
            }],
            required_ports: vec![],
        };

        let legacy = LegacySnmpCredentialMapping::from(&mapping);
        // FilePath default is skipped
        assert!(legacy.default_credential.is_none());
        // Inline override is preserved
        assert_eq!(legacy.ip_overrides.len(), 1);
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
        assert_eq!(
            creds[0].community.inline_value(),
            Some("override-community")
        );
        assert_eq!(creds[1].community.inline_value(), Some("default-community"));
        assert_eq!(creds[2].community.inline_value(), Some("public"));

        // IP without override: default, then public
        let creds = mapping.get_credentials_by_specificity(&other_ip);
        assert_eq!(creds.len(), 2);
        assert_eq!(creds[0].community.inline_value(), Some("default-community"));
        assert_eq!(creds[1].community.inline_value(), Some("public"));
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
        assert_eq!(creds[0].community.inline_value(), Some("public"));
    }
}

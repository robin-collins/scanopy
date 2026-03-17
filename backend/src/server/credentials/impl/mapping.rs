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
    #[serde(serialize_with = "redact_secret_string")]
    #[schema(value_type = String)]
    pub community: Secret<String>,
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
            community: Secret::from("public".to_string()),
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
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(tag = "mode")]
pub enum ResolvableSecret {
    Inline { value: String },
    FilePath { path: String },
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
            && !credentials
                .iter()
                .any(|c| c.community.expose_secret() == default.community.expose_secret())
        {
            credentials.push(default.clone());
        }

        // 3. "public" fallback (least specific)
        let public_community = "public";
        if !credentials
            .iter()
            .any(|c| c.community.expose_secret() == public_community)
        {
            credentials.push(SnmpQueryCredential::public_default());
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
            community: cred.community.expose_secret().clone(),
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
            community: Secret::from(community.to_string()),
        }
    }

    #[test]
    fn exposed_serialization_roundtrip() {
        let original = SnmpCredentialMapping {
            default_credential: Some(cred("my-secret")),
            ip_overrides: vec![],
            required_ports: vec![],
        };

        // Convert to exposed (plaintext), serialize to JSON, deserialize back
        let exposed = SnmpCredentialMappingExposed::from(&original);
        let json = serde_json::to_string(&exposed).unwrap();
        let roundtripped: SnmpCredentialMapping = serde_json::from_str(&json).unwrap();

        assert_eq!(
            roundtripped
                .default_credential
                .as_ref()
                .unwrap()
                .community
                .expose_secret(),
            "my-secret"
        );
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
        assert_eq!(creds[0].community.expose_secret(), "override-community");
        assert_eq!(creds[1].community.expose_secret(), "default-community");
        assert_eq!(creds[2].community.expose_secret(), "public");

        // IP without override: default, then public
        let creds = mapping.get_credentials_by_specificity(&other_ip);
        assert_eq!(creds.len(), 2);
        assert_eq!(creds[0].community.expose_secret(), "default-community");
        assert_eq!(creds[1].community.expose_secret(), "public");
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
        assert_eq!(creds[0].community.expose_secret(), "public");
    }
}

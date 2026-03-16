//! Generic credential mapping for discovery dispatch.
//!
//! The mapping types define how credentials are resolved per-IP during discovery.
//! `CredentialMapping<T>` is generic over the query credential type.

use crate::server::credentials::r#impl::types::SnmpVersion;
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
}

impl From<&SnmpCredentialMapping> for SnmpCredentialMappingExposed {
    fn from(mapping: &SnmpCredentialMapping) -> Self {
        Self {
            default_credential: mapping.default_credential.as_ref().map(Into::into),
            ip_overrides: mapping.ip_overrides.iter().map(Into::into).collect(),
        }
    }
}

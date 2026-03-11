use crate::server::snmp_credentials::r#impl::base::SnmpCredential;
use crate::server::snmp_credentials::r#impl::base::SnmpVersion;
use redact::Secret;
use secrecy::ExposeSecret;
use serde::Deserialize;
use serde::Serialize;
use serde::Serializer;
use std::net::IpAddr;
use utoipa::ToSchema;

/// Serializer that redacts a Secret<String> to "********"
fn redact_secret<S: Serializer>(
    _secret: &Secret<String>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str("********")
}

/// Minimal SNMP credential for daemon queries (version + community only)
/// Does not include organization_id, name, timestamps - just what's needed for SNMP queries
///
/// The community string is wrapped in `Secret` to prevent accidental exposure in logs,
/// debug output, and API responses. Use `community.expose_secret()` for explicit access
/// (e.g. daemon SNMP sessions).
#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Hash, Default, ToSchema)]
pub struct SnmpQueryCredential {
    /// SNMP version (V2c or V3)
    #[serde(default)]
    pub version: SnmpVersion,
    /// SNMPv2c community string — redacted in serialization/debug by default
    #[serde(serialize_with = "redact_secret")]
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

impl From<SnmpCredential> for SnmpQueryCredential {
    fn from(value: SnmpCredential) -> Self {
        Self {
            version: value.base.version,
            community: Secret::from(value.base.community.expose_secret().to_string()),
        }
    }
}

/// IP-specific SNMP credential override
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, ToSchema)]
pub struct SnmpIpOverride {
    /// IP address for this override
    #[schema(value_type = String)]
    pub ip: IpAddr,
    /// Credential to use for this IP
    pub credential: SnmpQueryCredential,
}

/// SNMP credential mapping for network discovery
/// Server builds this before initiating discovery; daemon uses it during scan
#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq, Hash, ToSchema)]
pub struct SnmpCredentialMapping {
    /// Network default credential (used when IP not in overrides)
    #[serde(default)]
    pub default_credential: Option<SnmpQueryCredential>,
    /// Per-IP overrides (from host.snmp_credential_id where host has known IPs)
    #[serde(default)]
    pub ip_overrides: Vec<SnmpIpOverride>,
}

impl SnmpCredentialMapping {
    /// Get credential for a specific IP, falling back to default
    pub fn get_credential_for_ip(&self, ip: &IpAddr) -> Option<SnmpQueryCredential> {
        self.ip_overrides
            .iter()
            .find(|o| &o.ip == ip)
            .map(|o| o.credential.clone())
            .or(self.default_credential.clone())
    }

    /// Get credentials for an IP ordered by specificity: IP override → network default → "public".
    /// Deduplicates by community string so we don't try the same credential twice.
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
            credentials.push(SnmpQueryCredential {
                version: Default::default(),
                community: Secret::from(public_community.to_string()),
            });
        }

        credentials
    }

    /// Check if SNMP is enabled (has at least a default or override)
    pub fn is_enabled(&self) -> bool {
        self.default_credential.is_some() || !self.ip_overrides.is_empty()
    }
}

// --- Exposed types for daemon serialization (plaintext community) ---

/// SNMP credential with community as plain String.
/// Used only for daemon serialization where the daemon needs actual credentials.
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

impl From<&SnmpIpOverride> for SnmpIpOverrideExposed {
    fn from(o: &SnmpIpOverride) -> Self {
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

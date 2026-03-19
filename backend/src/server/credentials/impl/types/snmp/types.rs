//! SNMP-specific credential types for discovery dispatch.

use crate::server::credentials::r#impl::mapping::{
    BannerField, BannerFieldValue, CredentialMapping, IpOverride, ResolvableSecret,
};
use crate::server::ports::r#impl::base::PortType;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use utoipa::ToSchema;

/// SNMP protocol version
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    Default,
    strum::VariantNames,
    ToSchema,
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

/// Banner lines for SNMP credentials
impl SnmpQueryCredential {
    pub fn banner_lines(&self) -> Vec<BannerField> {
        vec![
            BannerField {
                label: "Community",
                value: self.community.banner_value(),
            },
            BannerField {
                label: "Version",
                value: BannerFieldValue::Plain(self.version.to_string()),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::credentials::r#impl::mapping::CredentialQueryPayload;
    use std::net::IpAddr;
    use uuid::Uuid;

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
    fn specificity_ordering() {
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        let other_ip: IpAddr = "10.0.0.2".parse().unwrap();

        let mapping = SnmpCredentialMapping {
            default_credential: Some(cred("default-community")),
            ip_overrides: vec![IpOverride {
                ip,
                credential: cred("override-community"),
                credential_id: Uuid::nil(),
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
    fn specificity_deduplicates() {
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        // Override and default are both "public" — should not duplicate
        let mapping = SnmpCredentialMapping {
            default_credential: Some(cred("public")),
            ip_overrides: vec![IpOverride {
                ip,
                credential: cred("public"),
                credential_id: Uuid::nil(),
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
        assert!(matches!(
            lines[0].value,
            BannerFieldValue::RedactedInline(12)
        )); // "my-community".len()
        assert_eq!(lines[1].label, "Version");
        assert!(matches!(&lines[1].value, BannerFieldValue::Plain(v) if v == "V2c"));
    }
}

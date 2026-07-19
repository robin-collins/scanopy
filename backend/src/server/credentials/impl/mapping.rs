//! Generic credential mapping for discovery dispatch.
//!
//! The mapping types define how credentials are resolved per-IP during discovery.
//! `CredentialMapping<T>` is generic over the query credential type.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::{Path, PathBuf};
use strum::EnumDiscriminants;
use tempfile::NamedTempFile;
use utoipa::ToSchema;
use uuid::Uuid;

const MAX_SSH_SECRET_BYTES: usize = 64 * 1024;
const MAX_AD_SECRET_BYTES: usize = 64 * 1024;
const MAX_AD_CA_BYTES: usize = 1024 * 1024;
const MAX_UNIFI_SECRET_BYTES: usize = 64 * 1024;
const MAX_WINRM_SECRET_BYTES: usize = 64 * 1024;

// Re-export type-specific types so external imports don't break
pub use super::types::active_directory::{
    ActiveDirectoryKerberosQueryCredential, ActiveDirectoryLdapsQueryCredential,
};
pub use super::types::container_proxy::ContainerProxyQueryCredential;

/// Container-runtime (Docker/Podman) socket query credential. The daemon connects via a local
/// Unix socket; `socket_path` optionally repoints it (e.g. rootless Podman at
/// `$XDG_RUNTIME_DIR/podman/podman.sock`, a non-default `DOCKER_HOST`). Blank ⇒ the daemon
/// auto-detects (bollard defaults for Docker, `resolve_podman_socket_path()` for Podman).
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, Default)]
pub struct ContainerSocketQueryCredential {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub socket_path: Option<String>,
}
pub use super::types::snmp::{
    SnmpCredentialMapping, SnmpCredentialMappingExposed, SnmpIpOverrideExposed,
    SnmpQueryCredential, SnmpQueryCredentialExposed, SnmpV3AuthProtocol, SnmpV3Params,
    SnmpV3PrivProtocol, SnmpVersion,
};
pub use super::types::ssh::{SshAuthentication, SshHostKeyPolicy, SshPlatform, SshQueryCredential};
pub use super::types::unifi::{UnifiApiType, UnifiQueryCredential, UnifiTlsPolicy};
pub use super::types::winrm::{WindowsDomainAccountQueryCredential, WindowsLocalAccountQueryCredential};

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
    /// Credential ID for tracking which credential was used during discovery.
    #[serde(default)]
    pub credential_id: Uuid,
}

impl<T> IpOverride<T> {
    /// Check if this override targets localhost (127.0.0.1 or ::1).
    pub fn is_localhost(&self) -> bool {
        self.ip == IpAddr::V4(Ipv4Addr::LOCALHOST) || self.ip == IpAddr::V6(Ipv6Addr::LOCALHOST)
    }
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

    /// Collect all unique credential IDs referenced in this mapping's IP overrides.
    /// Excludes nil UUIDs (which indicate no server-side credential).
    pub fn credential_ids(&self) -> Vec<Uuid> {
        self.ip_overrides
            .iter()
            .map(|o| o.credential_id)
            .filter(|id| *id != Uuid::nil())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect()
    }
}

/// A credential payload paired with its server-side ID (if host-assignable).
/// `credential_id` is Some for host-scoped credentials (IP overrides from host assignments).
/// None for network-level defaults and fallbacks — those don't get auto-assigned
/// to discovered hosts because they're already available network-wide.
#[derive(Debug, Clone)]
pub struct ResolvedCredential<T> {
    pub credential: T,
    pub credential_id: Option<Uuid>,
}

/// Per-daemon integration targeting, stored on the `Discovery` entity and delivered via the
/// init command at registration. Each entry references exactly one stored credential and says
/// where it applies on this daemon. This is the single home for cred↔IP targeting — it replaces
/// the global, race-prone `credential.target_ips`.
///
/// The variants ARE the scopes; their strum [`Target`] discriminants are the capability enum that
/// `CredentialType::targets()` returns and validates against (single source of truth). Every
/// target carries a real `credential_id` — there is no credential-less branch and no nil
/// sentinel; a local socket is just a credential whose type targets only the daemon host.
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema, EnumDiscriminants,
)]
// `Target` is the capability enum returned by `CredentialType::targets()`: where a credential
// can apply (DaemonHost / Network / Hosts). It's the strum discriminant of `IntegrationTarget`.
#[strum_discriminants(
    name(Target),
    derive(Serialize, Deserialize, Hash, ToSchema, strum::VariantNames)
)]
#[serde(tag = "scope")]
pub enum IntegrationTarget {
    /// The daemon's own host — realized as a 127.0.0.1 IP-override (e.g. a local Docker/Podman
    /// socket, or any credential the user pins to the daemon host without naming its IP).
    DaemonHost { credential_id: Uuid },
    /// All hosts on the network — a broadcast default credential.
    Network { credential_id: Uuid },
    /// Specific host IPs — one IP-override per address.
    Hosts {
        credential_id: Uuid,
        #[schema(value_type = Vec<String>)]
        ips: Vec<IpAddr>,
    },
}

impl IntegrationTarget {
    /// The stored credential this target references (present in every variant).
    pub fn credential_id(&self) -> Uuid {
        match self {
            Self::DaemonHost { credential_id }
            | Self::Network { credential_id }
            | Self::Hosts { credential_id, .. } => *credential_id,
        }
    }
}

// ============================================================================
// Generic Credential Query Types (wire format for unified discovery)
// ============================================================================

/// Credential payload sent to daemon with secrets exposed.
/// Each variant corresponds to a CredentialType variant.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, EnumDiscriminants)]
#[strum_discriminants(derive(Hash, strum::Display))]
#[serde(tag = "type")]
pub enum CredentialQueryPayload {
    Snmp(SnmpQueryCredential),
    Ssh(SshQueryCredential),
    ActiveDirectoryLdaps(ActiveDirectoryLdapsQueryCredential),
    ActiveDirectoryKerberos(ActiveDirectoryKerberosQueryCredential),
    Unifi(UnifiQueryCredential),
    DockerProxy(ContainerProxyQueryCredential),
    DockerSocket(ContainerSocketQueryCredential),
    PodmanProxy(ContainerProxyQueryCredential),
    PodmanSocket(ContainerSocketQueryCredential),
    WindowsLocalAccount(WindowsLocalAccountQueryCredential),
    WindowsDomainAccount(WindowsDomainAccountQueryCredential),
    /// Forward-compat fallback: a credential type from a newer server that this
    /// daemon doesn't recognize. `#[serde(other)]` deserializes any unknown `type`
    /// tag here (a unit variant, the only shape allowed for `other` on an
    /// internally-tagged enum — mirrors `EntitySource`/`SubnetType`) instead of
    /// hard-failing the whole discovery request. The daemon's dispatch skips it.
    #[serde(other)]
    Unknown,
}

impl Default for CredentialQueryPayload {
    fn default() -> Self {
        Self::Snmp(SnmpQueryCredential::default())
    }
}

impl From<CredentialQueryPayloadDiscriminants> for super::types::CredentialTypeDiscriminants {
    fn from(d: CredentialQueryPayloadDiscriminants) -> Self {
        match d {
            CredentialQueryPayloadDiscriminants::Snmp => Self::SnmpV2c,
            CredentialQueryPayloadDiscriminants::Ssh => Self::SshPassword,
            CredentialQueryPayloadDiscriminants::ActiveDirectoryLdaps => Self::ActiveDirectoryLdaps,
            CredentialQueryPayloadDiscriminants::ActiveDirectoryKerberos => {
                Self::ActiveDirectoryKerberos
            }
            CredentialQueryPayloadDiscriminants::Unifi => Self::UnifiPassword,
            CredentialQueryPayloadDiscriminants::DockerProxy => Self::DockerProxy,
            CredentialQueryPayloadDiscriminants::DockerSocket => Self::DockerSocket,
            CredentialQueryPayloadDiscriminants::PodmanProxy => Self::PodmanProxy,
            CredentialQueryPayloadDiscriminants::PodmanSocket => Self::PodmanSocket,
            CredentialQueryPayloadDiscriminants::WindowsLocalAccount => Self::WindowsLocalAccount,
            CredentialQueryPayloadDiscriminants::WindowsDomainAccount => Self::WindowsDomainAccount,
            // `Unknown` is the daemon-side forward-compat sentinel; the server only
            // ever builds `CredentialQueryPayload` from a known `CredentialType`, so
            // this reverse conversion never sees it. Fall back to the SNMP default to
            // keep the mapping total (unreachable server-side).
            CredentialQueryPayloadDiscriminants::Unknown => Self::SnmpV2c,
        }
    }
}

impl CredentialQueryPayload {
    /// The proxy credential for either container-runtime proxy variant
    /// (Docker/Podman), which share the same Docker-compatible API shape.
    pub fn as_container_proxy(&self) -> Option<&ContainerProxyQueryCredential> {
        match self {
            Self::DockerProxy(c) | Self::PodmanProxy(c) => Some(c),
            _ => None,
        }
    }

    /// Ports that should be included in light scans for this credential type.
    /// Used by network scanning to ensure integration-relevant ports are always scanned.
    pub fn required_scan_ports(&self) -> Vec<u16> {
        match self {
            Self::Snmp(_) => vec![161, 1161],
            Self::Ssh(ssh) => vec![ssh.port],
            Self::ActiveDirectoryLdaps(ad) => vec![ad.port],
            Self::ActiveDirectoryKerberos(ad) => vec![ad.port],
            Self::Unifi(unifi) => unifi.port().into_iter().collect(),
            Self::DockerProxy(d) | Self::PodmanProxy(d) => vec![d.port],
            Self::DockerSocket(_) | Self::PodmanSocket(_) => vec![],
            Self::WindowsLocalAccount(w) => vec![w.port],
            Self::WindowsDomainAccount(w) => vec![w.port],
            Self::Unknown => vec![],
        }
    }

    pub fn discovery_label(&self) -> &'static str {
        match self {
            Self::Snmp(_) => "SNMP queries",
            Self::Ssh(_) => "SSH read-only collection",
            Self::ActiveDirectoryLdaps(_) => "Active Directory LDAPS collection",
            Self::ActiveDirectoryKerberos(_) => "Active Directory Kerberos LDAPS collection",
            Self::Unifi(_) => "UniFi controller collection",
            Self::DockerProxy(_) => "Docker proxy connection",
            Self::DockerSocket(_) => "Docker socket connection",
            Self::PodmanProxy(_) => "Podman proxy connection",
            Self::PodmanSocket(_) => "Podman socket connection",
            Self::WindowsLocalAccount(_) => "WinRM local account collection",
            Self::WindowsDomainAccount(_) => "WinRM domain account collection",
            Self::Unknown => "unknown credential",
        }
    }

    /// Resolve all FilePath fields to Value by reading from disk,
    /// then validate PEM contents for fields that require it.
    pub fn resolve_file_paths(&self) -> Result<Self, anyhow::Error> {
        use super::types::InlineFormat;

        let label = self.discovery_label();
        match self {
            Self::Snmp(snmp) => {
                let v3 = snmp
                    .v3
                    .as_ref()
                    .map(|v3| -> Result<_, anyhow::Error> {
                        Ok(super::types::snmp::SnmpV3Params {
                            security_name: v3.security_name.clone(),
                            auth_protocol: v3.auth_protocol,
                            auth_password: v3
                                .auth_password
                                .resolve_to_value("auth_password", label)?,
                            priv_protocol: v3.priv_protocol,
                            priv_password: v3
                                .priv_password
                                .resolve_to_value("priv_password", label)?,
                            context_name: v3.context_name.clone(),
                        })
                    })
                    .transpose()?;
                Ok(Self::Snmp(SnmpQueryCredential {
                    version: snmp.version,
                    community: snmp.community.resolve_to_value("community", label)?,
                    v3,
                }))
            }
            Self::Ssh(ssh) => {
                let authentication = match &ssh.authentication {
                    SshAuthentication::Password { password } => SshAuthentication::Password {
                        password: password.resolve_to_value_bounded(
                            "password",
                            label,
                            MAX_SSH_SECRET_BYTES,
                        )?,
                    },
                    SshAuthentication::PrivateKey {
                        private_key,
                        passphrase,
                    } => SshAuthentication::PrivateKey {
                        private_key: private_key.resolve_to_value_bounded(
                            "private_key",
                            label,
                            MAX_SSH_SECRET_BYTES,
                        )?,
                        passphrase: passphrase
                            .as_ref()
                            .map(|value| {
                                value.resolve_to_value_bounded(
                                    "passphrase",
                                    label,
                                    MAX_SSH_SECRET_BYTES,
                                )
                            })
                            .transpose()?,
                    },
                };
                Ok(Self::Ssh(SshQueryCredential {
                    username: ssh.username.clone(),
                    authentication,
                    port: ssh.port,
                    platform: ssh.platform,
                    host_key_policy: ssh.host_key_policy,
                    known_hosts_file: ssh.known_hosts_file.clone(),
                }))
            }
            Self::ActiveDirectoryLdaps(ad) => Ok(Self::ActiveDirectoryLdaps(
                ActiveDirectoryLdapsQueryCredential {
                    bind_dn: ad.bind_dn.clone(),
                    password: ad.password.resolve_to_value_bounded(
                        "password",
                        label,
                        MAX_AD_SECRET_BYTES,
                    )?,
                    port: ad.port,
                    server_name: ad.server_name.clone(),
                    base_dn: ad.base_dn.clone(),
                    ca_certificate: ad
                        .ca_certificate
                        .as_ref()
                        .map(|value| {
                            value.resolve_to_value_bounded("ca_certificate", label, MAX_AD_CA_BYTES)
                        })
                        .transpose()?,
                    group_dns: ad.group_dns.clone(),
                },
            )),
            Self::ActiveDirectoryKerberos(ad) => Ok(Self::ActiveDirectoryKerberos(
                ActiveDirectoryKerberosQueryCredential {
                    principal: ad.principal.clone(),
                    use_system_ccache: ad.use_system_ccache,
                    port: ad.port,
                    server_name: ad.server_name.clone(),
                    base_dn: ad.base_dn.clone(),
                    ca_certificate: ad
                        .ca_certificate
                        .as_ref()
                        .map(|value| {
                            value.resolve_to_value_bounded("ca_certificate", label, MAX_AD_CA_BYTES)
                        })
                        .transpose()?,
                    group_dns: ad.group_dns.clone(),
                },
            )),
            Self::Unifi(unifi) => Ok(Self::Unifi(UnifiQueryCredential {
                controller_url: unifi.controller_url.clone(),
                server_name: unifi.server_name.clone(),
                site: unifi.site.clone(),
                api_type: unifi.api_type,
                tls_policy: unifi.tls_policy,
                username: unifi.username.clone(),
                password: unifi.password.resolve_to_value_bounded(
                    "password",
                    label,
                    MAX_UNIFI_SECRET_BYTES,
                )?,
            })),
            Self::DockerProxy(d) | Self::PodmanProxy(d) => {
                let ssl_cert = d
                    .ssl_cert
                    .as_ref()
                    .map(|v| v.resolve_to_value("ssl_cert", label))
                    .transpose()?;
                let ssl_key = d
                    .ssl_key
                    .as_ref()
                    .map(|v| v.resolve_to_value("ssl_key", label))
                    .transpose()?;
                let ssl_chain = d
                    .ssl_chain
                    .as_ref()
                    .map(|v| v.resolve_to_value("ssl_chain", label))
                    .transpose()?;

                // Validate resolved PEM contents
                if let Some(ResolvableValue::Value { value }) = &ssl_cert {
                    InlineFormat::PemCertificate.validate(value, "SSL Certificate")?;
                }
                if let Some(ResolvableSecret::Value { value }) = &ssl_key {
                    InlineFormat::PemPrivateKey.validate(value, "SSL Private Key")?;
                }
                if let Some(ResolvableValue::Value { value }) = &ssl_chain {
                    InlineFormat::PemCertificate.validate(value, "SSL CA Chain")?;
                }

                let resolved = ContainerProxyQueryCredential {
                    port: d.port,
                    path: d.path.clone(),
                    ssl_cert,
                    ssl_key,
                    ssl_chain,
                };
                Ok(match self {
                    Self::PodmanProxy(_) => Self::PodmanProxy(resolved),
                    _ => Self::DockerProxy(resolved),
                })
            }
            Self::DockerSocket(d) => Ok(Self::DockerSocket(d.clone())),
            Self::PodmanSocket(d) => Ok(Self::PodmanSocket(d.clone())),
            Self::WindowsLocalAccount(w) => Ok(Self::WindowsLocalAccount(
                WindowsLocalAccountQueryCredential {
                    username: w.username.clone(),
                    password: w.password.resolve_to_value_bounded(
                        "password",
                        label,
                        MAX_WINRM_SECRET_BYTES,
                    )?,
                    port: w.port,
                    use_tls: w.use_tls,
                    accept_invalid_certs: w.accept_invalid_certs,
                },
            )),
            Self::WindowsDomainAccount(w) => Ok(Self::WindowsDomainAccount(
                WindowsDomainAccountQueryCredential {
                    domain: w.domain.clone(),
                    username: w.username.clone(),
                    password: w.password.resolve_to_value_bounded(
                        "password",
                        label,
                        MAX_WINRM_SECRET_BYTES,
                    )?,
                    port: w.port,
                    use_tls: w.use_tls,
                    accept_invalid_certs: w.accept_invalid_certs,
                },
            )),
            Self::Unknown => Ok(Self::Unknown),
        }
    }

    pub fn banner_lines(&self) -> Vec<BannerField> {
        match self {
            Self::Snmp(snmp) => snmp.banner_lines(),
            Self::Ssh(_) => vec![],
            Self::ActiveDirectoryLdaps(_) => vec![],
            Self::ActiveDirectoryKerberos(_) => vec![],
            Self::Unifi(_) => vec![],
            Self::DockerProxy(c) | Self::PodmanProxy(c) => c.banner_lines(),
            Self::DockerSocket(_) | Self::PodmanSocket(_) => vec![],
            Self::WindowsLocalAccount(_) | Self::WindowsDomainAccount(_) => vec![],
            Self::Unknown => vec![],
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
///
/// Custom Deserialize accepts both the current tagged-enum format
/// (`{"mode":"Value","value":"..."}`) and legacy plain strings (`"********"`)
/// from pre-v0.15.0 discovery_type JSONB. Legacy strings deserialize as
/// `Value { value: string }`.
#[derive(Debug, Clone, Serialize, Eq, PartialEq, Hash, ToSchema)]
#[serde(tag = "mode")]
pub enum ResolvableSecret {
    Value { value: String },
    FilePath { path: String },
}

impl<'de> Deserialize<'de> for ResolvableSecret {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match &value {
            serde_json::Value::String(s) => Ok(ResolvableSecret::Value { value: s.clone() }),
            serde_json::Value::Object(_) => {
                #[derive(Deserialize)]
                #[serde(tag = "mode")]
                enum Tagged {
                    Value { value: String },
                    FilePath { path: String },
                }
                let tagged: Tagged =
                    serde_json::from_value(value).map_err(serde::de::Error::custom)?;
                Ok(match tagged {
                    Tagged::Value { value } => ResolvableSecret::Value { value },
                    Tagged::FilePath { path } => ResolvableSecret::FilePath { path },
                })
            }
            _ => Err(serde::de::Error::custom(
                "expected string or object for ResolvableSecret",
            )),
        }
    }
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

    fn resolve_to_value_bounded(
        &self,
        field_name: &str,
        label: &str,
        max_bytes: usize,
    ) -> Result<Self, anyhow::Error> {
        match self {
            Self::Value { value } => {
                if value.len() > max_bytes {
                    anyhow::bail!(
                        "{} for {} exceeds the {} byte limit",
                        field_name,
                        label,
                        max_bytes
                    );
                }
                Ok(self.clone())
            }
            Self::FilePath { path } => {
                tracing::info!("Read {} from {} for {}", field_name, path, label);
                let file = std::fs::File::open(path)?;
                let mut contents = String::new();
                file.take(max_bytes as u64 + 1)
                    .read_to_string(&mut contents)?;
                if contents.len() > max_bytes {
                    anyhow::bail!(
                        "{} for {} exceeds the {} byte limit",
                        field_name,
                        label,
                        max_bytes
                    );
                }
                Ok(Self::Value { value: contents })
            }
        }
    }

    /// Resolve to a filesystem path. FilePath returns the path directly.
    /// Value writes content to a temp file (caller must hold the handle to keep it alive).
    pub fn resolve_to_path(
        &self,
        field_name: &str,
        label: &str,
    ) -> Result<(PathBuf, Option<NamedTempFile>), anyhow::Error> {
        match self {
            Self::FilePath { path } => Ok((PathBuf::from(path), None)),
            Self::Value { value } => {
                let mut tmp = NamedTempFile::new().map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to create temp file for {} ({}): {}",
                        field_name,
                        label,
                        e
                    )
                })?;
                tmp.write_all(value.as_bytes()).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to write {} to temp file for {}: {}",
                        field_name,
                        label,
                        e
                    )
                })?;
                tmp.flush()?;
                let path = tmp.path().to_path_buf();
                Ok((path, Some(tmp)))
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

    fn resolve_to_value_bounded(
        &self,
        field_name: &str,
        label: &str,
        max_bytes: usize,
    ) -> Result<Self, anyhow::Error> {
        match self {
            Self::Value { value } => {
                if value.len() > max_bytes {
                    anyhow::bail!(
                        "{} for {} exceeds the {} byte limit",
                        field_name,
                        label,
                        max_bytes
                    );
                }
                Ok(self.clone())
            }
            Self::FilePath { path } => {
                tracing::info!("Read {} (********) from {} for {}", field_name, path, label);
                let file = std::fs::File::open(path).map_err(|error| {
                    anyhow::anyhow!(
                        "Failed to open {} from {} for {}: {}",
                        field_name,
                        path,
                        label,
                        error
                    )
                })?;
                let mut contents = String::new();
                file.take(max_bytes as u64 + 1)
                    .read_to_string(&mut contents)
                    .map_err(|error| {
                        anyhow::anyhow!(
                            "Failed to read {} from {} for {}: {}",
                            field_name,
                            path,
                            label,
                            error
                        )
                    })?;
                if contents.len() > max_bytes {
                    anyhow::bail!(
                        "{} from {} for {} exceeds the {} byte limit",
                        field_name,
                        path,
                        label,
                        max_bytes
                    );
                }
                Ok(Self::Value { value: contents })
            }
        }
    }

    /// Resolve to a filesystem path. FilePath returns the path directly.
    /// Value writes content to a temp file (caller must hold the handle to keep it alive).
    pub fn resolve_to_path(
        &self,
        field_name: &str,
        label: &str,
    ) -> Result<(PathBuf, Option<NamedTempFile>), anyhow::Error> {
        match self {
            Self::FilePath { path } => Ok((PathBuf::from(path), None)),
            Self::Value { value } => {
                let mut tmp = NamedTempFile::new().map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to create temp file for {} ({}): {}",
                        field_name,
                        label,
                        e
                    )
                })?;
                tmp.write_all(value.as_bytes()).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to write {} to temp file for {}: {}",
                        field_name,
                        label,
                        e
                    )
                })?;
                tmp.flush()?;
                let path = tmp.path().to_path_buf();
                Ok((path, Some(tmp)))
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
    /// Long inline value — show "<inline, N chars>" instead of dumping content
    InlineSummary(usize),
    /// Inline secret — show "******** (N chars)"
    RedactedInline(usize),
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
            Self::InlineSummary(len) => write!(f, "<inline, {} chars>", len),
            Self::RedactedInline(len) => write!(f, "******** ({} chars)", len),
            Self::FileOk(path) => write!(f, "successfully read from {}", path),
            Self::FileFailed(path) => write!(f, "failed to read from {}", path),
        }
    }
}

impl ResolvableValue {
    pub fn banner_value(&self) -> BannerFieldValue {
        match self {
            Self::Value { value } => {
                if value.len() > 64 {
                    BannerFieldValue::InlineSummary(value.len())
                } else {
                    BannerFieldValue::Plain(value.clone())
                }
            }
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
            Self::Value { value } => BannerFieldValue::RedactedInline(value.len()),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_snmp_cred(community: &str) -> SnmpQueryCredential {
        SnmpQueryCredential {
            version: SnmpVersion::V2c,
            community: ResolvableSecret::Value {
                value: community.to_string(),
            },
            v3: None,
        }
    }

    #[test]
    fn ssh_secret_file_resolution_enforces_size_limit() {
        let directory = tempfile::tempdir().unwrap();
        let secret_path = directory.path().join("oversized-secret");
        std::fs::write(&secret_path, "x".repeat(MAX_SSH_SECRET_BYTES + 1)).unwrap();
        let payload = CredentialQueryPayload::Ssh(SshQueryCredential {
            username: "scanopy".to_string(),
            authentication: SshAuthentication::Password {
                password: ResolvableSecret::FilePath {
                    path: secret_path.to_string_lossy().into_owned(),
                },
            },
            port: 22,
            platform: SshPlatform::Linux,
            host_key_policy: SshHostKeyPolicy::AcceptUnknown,
            known_hosts_file: None,
        });

        let error = payload
            .resolve_file_paths()
            .expect_err("oversized SSH secret files must be rejected");
        assert!(error.to_string().contains("65536 byte limit"));
        assert!(!error.to_string().contains(&"x".repeat(64)));
    }

    fn make_override(ip: IpAddr, cred_id: Uuid) -> IpOverride<SnmpQueryCredential> {
        IpOverride {
            ip,
            credential: make_snmp_cred("public"),
            credential_id: cred_id,
        }
    }

    // -- credential_ids --

    #[test]
    fn credential_ids_filters_nil_uuids() {
        let mapping = CredentialMapping {
            default_credential: Some(make_snmp_cred("public")),
            ip_overrides: vec![
                make_override("10.0.0.1".parse().unwrap(), Uuid::nil()),
                make_override("10.0.0.2".parse().unwrap(), Uuid::new_v4()),
            ],
        };
        let ids = mapping.credential_ids();
        assert_eq!(ids.len(), 1);
        assert_ne!(ids[0], Uuid::nil());
    }

    #[test]
    fn credential_ids_deduplicates() {
        let shared_id = Uuid::new_v4();
        let mapping = CredentialMapping {
            default_credential: None,
            ip_overrides: vec![
                make_override("10.0.0.1".parse().unwrap(), shared_id),
                make_override("10.0.0.2".parse().unwrap(), shared_id),
            ],
        };
        let ids = mapping.credential_ids();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], shared_id);
    }

    #[test]
    fn credential_ids_empty_when_no_overrides() {
        let mapping: CredentialMapping<SnmpQueryCredential> = CredentialMapping {
            default_credential: Some(make_snmp_cred("public")),
            ip_overrides: vec![],
        };
        assert!(mapping.credential_ids().is_empty());
    }

    // -- is_enabled --

    #[test]
    fn is_enabled_default_only() {
        let mapping = CredentialMapping {
            default_credential: Some(make_snmp_cred("public")),
            ip_overrides: vec![],
        };
        assert!(mapping.is_enabled());
    }

    #[test]
    fn is_enabled_overrides_only() {
        let mapping = CredentialMapping {
            default_credential: None,
            ip_overrides: vec![make_override("10.0.0.1".parse().unwrap(), Uuid::new_v4())],
        };
        assert!(mapping.is_enabled());
    }

    #[test]
    fn is_enabled_empty() {
        let mapping: CredentialMapping<SnmpQueryCredential> = CredentialMapping::default();
        assert!(!mapping.is_enabled());
    }

    // -- get_credential_for_ip --

    #[test]
    fn get_credential_for_ip_override_match() {
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        let mapping = CredentialMapping {
            default_credential: Some(make_snmp_cred("default")),
            ip_overrides: vec![IpOverride {
                ip,
                credential: make_snmp_cred("override"),
                credential_id: Uuid::new_v4(),
            }],
        };
        let cred = mapping.get_credential_for_ip(&ip).unwrap();
        assert_eq!(
            cred.community,
            ResolvableSecret::Value {
                value: "override".to_string()
            }
        );
    }

    #[test]
    fn get_credential_for_ip_fallback_to_default() {
        let mapping = CredentialMapping {
            default_credential: Some(make_snmp_cred("default")),
            ip_overrides: vec![make_override("10.0.0.1".parse().unwrap(), Uuid::new_v4())],
        };
        let other_ip: IpAddr = "10.0.0.99".parse().unwrap();
        let cred = mapping.get_credential_for_ip(&other_ip).unwrap();
        assert_eq!(
            cred.community,
            ResolvableSecret::Value {
                value: "default".to_string()
            }
        );
    }

    #[test]
    fn get_credential_for_ip_no_match() {
        let mapping: CredentialMapping<SnmpQueryCredential> = CredentialMapping {
            default_credential: None,
            ip_overrides: vec![make_override("10.0.0.1".parse().unwrap(), Uuid::new_v4())],
        };
        let other_ip: IpAddr = "10.0.0.99".parse().unwrap();
        assert!(mapping.get_credential_for_ip(&other_ip).is_none());
    }

    // -- is_localhost --

    #[test]
    fn is_localhost_v4() {
        let o = make_override("127.0.0.1".parse().unwrap(), Uuid::new_v4());
        assert!(o.is_localhost());
    }

    #[test]
    fn is_localhost_v6() {
        let o = make_override("::1".parse().unwrap(), Uuid::new_v4());
        assert!(o.is_localhost());
    }

    #[test]
    fn is_localhost_non_local() {
        let o = make_override("10.0.0.1".parse().unwrap(), Uuid::new_v4());
        assert!(!o.is_localhost());
    }
}

use crate::server::{
    credentials::r#impl::mapping::{
        ContainerProxyQueryCredential, ContainerSocketQueryCredential, CredentialQueryPayload,
        ResolvableSecret, ResolvableValue,
    },
    ports::r#impl::base::PortType,
    services::r#impl::definitions::ServiceDefinition,
};
use anyhow::Error;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumDiscriminants, EnumIter};
use strum_macros::{IntoStaticStr, VariantNames};
use utoipa::ToSchema;

pub mod active_directory;
pub mod container_proxy;
pub mod snmp;
pub mod ssh;
pub mod unifi;
pub mod winrm;

mod fields;
mod metadata;
mod secrets;

pub use active_directory::{
    ActiveDirectoryKerberosQueryCredential, ActiveDirectoryLdapsQueryCredential, default_ldaps_port,
};
pub use fields::{FieldDefinition, FieldType, InlineFormat, PemTag, SelectOption};
pub use metadata::{
    CredentialAssignment, CredentialCategory, CredentialHostAssignment, CredentialStability,
};
// `Target` is the strum-discriminant of `IntegrationTarget` (single source of truth for the
// scope scheme); re-export it here so `CredentialType::targets()` and existing imports resolve.
pub use super::mapping::Target;
pub use secrets::{
    ExposeSecretsGuard, FileOrInline, REDACTED_SECRET_SENTINEL, SecretValue,
    deserialize_optional_file_or_inline, deserialize_optional_secret_value,
};

// Re-export SnmpVersion and v3 protocol enums from snmp submodule
pub use snmp::{SnmpV3AuthProtocol, SnmpV3PrivProtocol, SnmpVersion};
pub use ssh::{SshAuthentication, SshHostKeyPolicy, SshPlatform, SshQueryCredential};
pub use unifi::{UnifiAuth, UnifiQueryCredential, default_unifi_port, default_unifi_site};
pub use winrm::{
    WindowsDomainAccountQueryCredential, WindowsLocalAccountQueryCredential, default_winrm_port,
};

fn default_docker_port() -> u16 {
    PortType::Docker.number()
}

fn default_ssh_port() -> u16 {
    PortType::Ssh.number()
}

fn is_supported_absolute_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    path.starts_with('/')
        || path.starts_with("\\\\")
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'/' | b'\\'))
}

const MAX_AD_DN_CHARS: usize = 2_048;
const MAX_AD_GROUP_SCOPE_CHARS: usize = 32_768;
const MAX_AD_GROUPS: usize = 16;

fn is_valid_ad_dn(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.chars().count() <= MAX_AD_DN_CHARS
        && value.contains('=')
        && !value.chars().any(char::is_control)
}

fn is_valid_dns_name(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.len() <= 253
        && !value.ends_with('.')
        && value.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

fn is_valid_kerberos_principal(value: &str) -> bool {
    let value = value.trim();
    let Some((name, realm)) = value.rsplit_once('@') else {
        return false;
    };
    !name.is_empty()
        && !realm.is_empty()
        && value.len() <= 1_024
        && !value.chars().any(|c| c.is_control() || c.is_whitespace())
        && realm
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

/// Universal credential type — tagged enum stored as JSONB.
/// Each variant represents a different credential protocol/method.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    ToSchema,
    EnumDiscriminants,
    IntoStaticStr,
    VariantNames,
)]
#[strum_discriminants(derive(
    Display,
    Hash,
    Serialize,
    Deserialize,
    IntoStaticStr,
    EnumIter,
    utoipa::ToSchema
))]
#[serde(tag = "type")]
pub enum CredentialType {
    /// SNMPv1 community string — for legacy devices that only speak v1.
    #[schema(title = "SnmpV1")]
    SnmpV1 {
        /// SNMPv1 community string.
        community: SecretValue,
    },
    /// SNMPv2c community string for querying network devices
    #[schema(title = "SnmpV2c")]
    SnmpV2c {
        /// SNMPv2c community string.
        community: SecretValue,
    },
    /// SNMPv3 USM AuthPriv — security name + auth/priv protocols and passwords.
    #[schema(title = "SnmpV3")]
    SnmpV3 {
        /// USM security (user) name.
        security_name: String,
        /// Hash algorithm used for authentication.
        auth_protocol: SnmpV3AuthProtocol,
        /// Authentication passphrase.
        auth_password: SecretValue,
        /// Cipher used for privacy (encryption).
        priv_protocol: SnmpV3PrivProtocol,
        /// Privacy passphrase.
        priv_password: SecretValue,
        /// Optional context name (default/empty context used if unset).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        context_name: Option<String>,
    },
    /// Read-only SSH collection using password authentication.
    SshPassword {
        username: String,
        password: SecretValue,
        #[serde(default = "default_ssh_port")]
        port: u16,
        platform: SshPlatform,
        host_key_policy: SshHostKeyPolicy,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        known_hosts_file: Option<String>,
    },
    /// Read-only SSH collection using an OpenSSH private key.
    SshPrivateKey {
        username: String,
        private_key: SecretValue,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        passphrase: Option<SecretValue>,
        #[serde(default = "default_ssh_port")]
        port: u16,
        platform: SshPlatform,
        host_key_policy: SshHostKeyPolicy,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        known_hosts_file: Option<String>,
    },
    /// Read-only Active Directory collection using password authentication over
    /// certificate-verified LDAPS. Plain LDAP and TLS bypasses are intentionally
    /// not represented by this credential type.
    ActiveDirectoryLdaps {
        bind_dn: String,
        password: SecretValue,
        #[serde(default = "default_ldaps_port")]
        port: u16,
        server_name: String,
        base_dn: String,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "deserialize_optional_file_or_inline"
        )]
        ca_certificate: Option<FileOrInline>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        group_dns: Option<String>,
    },
    /// Read-only Active Directory collection over certificate-verified LDAPS,
    /// authenticated with a specifically named principal from the daemon's
    /// external system credential cache. Scanopy never stores or mutates a
    /// password, keytab, ticket, or credential cache for this transport.
    ActiveDirectoryKerberos {
        principal: String,
        /// Explicit acknowledgement of the external read-only cache contract.
        /// Validation requires this to be exactly `true`.
        use_system_ccache: bool,
        #[serde(default = "default_ldaps_port")]
        port: u16,
        server_name: String,
        base_dn: String,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "deserialize_optional_file_or_inline"
        )]
        ca_certificate: Option<FileOrInline>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        group_dns: Option<String>,
    },
    /// Docker API proxy credentials. Target IP determined from host ip_addresses at scan time.
    #[schema(title = "DockerProxy")]
    DockerProxy {
        /// Port for the Docker API proxy (default 2375)
        #[serde(default = "default_docker_port")]
        port: u16,
        /// Optional URL path prefix (e.g. "/v1.43")
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        /// PEM-encoded public certificate — inline or file path on daemon host
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "deserialize_optional_file_or_inline"
        )]
        ssl_cert: Option<FileOrInline>,
        /// Private key — inline PEM content or file path on daemon host
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "deserialize_optional_secret_value"
        )]
        ssl_key: Option<SecretValue>,
        /// PEM-encoded CA chain — inline or file path on daemon host
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "deserialize_optional_file_or_inline"
        )]
        ssl_chain: Option<FileOrInline>,
    },
    /// Local Docker socket access on the daemon host. `socket_path` optionally repoints the
    /// socket (non-default `DOCKER_HOST`); blank ⇒ the daemon auto-detects (bollard defaults).
    #[schema(title = "DockerSocket")]
    DockerSocket {
        /// Path to the Docker socket. Blank lets the daemon auto-detect it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        socket_path: Option<String>,
    },
    /// Podman API proxy credentials. Podman exposes a Docker-compatible REST API,
    /// so the fields mirror `DockerProxy`. Target IP determined from host
    /// ip_addresses at scan time.
    #[schema(title = "PodmanProxy")]
    PodmanProxy {
        /// Port for the Podman API proxy (default 2375)
        #[serde(default = "default_docker_port")]
        port: u16,
        /// Optional URL path prefix (e.g. "/v1.43")
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        /// PEM-encoded public certificate — inline or file path on daemon host
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "deserialize_optional_file_or_inline"
        )]
        ssl_cert: Option<FileOrInline>,
        /// Private key — inline PEM content or file path on daemon host
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "deserialize_optional_secret_value"
        )]
        ssl_key: Option<SecretValue>,
        /// PEM-encoded CA chain — inline or file path on daemon host
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "deserialize_optional_file_or_inline"
        )]
        ssl_chain: Option<FileOrInline>,
    },
    /// Local Podman socket access on the daemon host. `socket_path` optionally repoints the
    /// socket (e.g. rootful `/run/podman/podman.sock` vs rootless
    /// `$XDG_RUNTIME_DIR/podman/podman.sock`); blank ⇒ the daemon auto-detects via
    /// `resolve_podman_socket_path()`.
    #[schema(title = "PodmanSocket")]
    PodmanSocket {
        /// Path to the Podman socket. Blank lets the daemon auto-detect it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        socket_path: Option<String>,
    },
    /// UniFi Network Application (controller) via an API key.
    ///
    /// **UniFi OS only** — a UniFi OS console (443) or UniFi OS Server (11443). The legacy
    /// self-hosted Network Application on 8443 does not support API keys; use
    /// [`CredentialType::UnifiLocalAdmin`] there.
    #[schema(title = "UnifiApiKey")]
    UnifiApiKey {
        /// Controller HTTPS port. 443 for a UniFi OS console, 11443 for UniFi OS Server.
        #[serde(default = "default_unifi_port")]
        port: u16,
        /// Internal site name from the controller URL (`/manage/site/<name>`).
        #[serde(default = "default_unifi_site")]
        site: String,
        /// Network Application API key, sent as `X-API-KEY`.
        api_key: SecretValue,
    },
    /// UniFi Network Application (controller) via a local-admin account.
    ///
    /// Works on every controller type, including the legacy self-hosted Network Application on
    /// 8443. Use a local-only admin account so MFA does not block the login.
    #[schema(title = "UnifiLocalAdmin")]
    UnifiLocalAdmin {
        /// Controller HTTPS port. 443 UniFi OS console, 11443 UniFi OS Server, 8443 legacy.
        #[serde(default = "default_unifi_port")]
        port: u16,
        /// Internal site name from the controller URL (`/manage/site/<name>`).
        #[serde(default = "default_unifi_site")]
        site: String,
        /// Local admin account on the controller.
        username: String,
        /// Password for that account.
        password: SecretValue,
    },
    /// Read-only Windows inventory collection over WinRM using a machine-local
    /// administrator account. Authenticates with NTLMv2; no domain/Kerberos
    /// infrastructure required. See `daemon::discovery::integration::winrm`
    /// for the encryption constraints this implies.
    WindowsLocalAccount {
        username: String,
        password: SecretValue,
        #[serde(default = "default_winrm_port")]
        port: u16,
        #[serde(default)]
        use_tls: bool,
        #[serde(default)]
        accept_invalid_certs: bool,
    },
    /// Read-only Windows inventory collection over WinRM using a domain
    /// account, authenticated with NTLM and an explicit domain qualifier
    /// rather than a Kerberos ticket.
    WindowsDomainAccount {
        domain: String,
        username: String,
        password: SecretValue,
        #[serde(default = "default_winrm_port")]
        port: u16,
        #[serde(default)]
        use_tls: bool,
        #[serde(default)]
        accept_invalid_certs: bool,
    },
}

/// Convert a stored `SecretValue` into a daemon-bound `ResolvableSecret`,
/// exposing inline secrets and passing through file paths.
fn secret_to_resolvable(secret: &SecretValue) -> ResolvableSecret {
    match secret {
        SecretValue::Inline { value } => ResolvableSecret::Value {
            value: value.expose_secret().to_string(),
        },
        SecretValue::FilePath { path } => ResolvableSecret::FilePath { path: path.clone() },
    }
}

/// Extract the inline string value of a secret, or `None` for file-path mode
/// or the redacted sentinel.
fn inline_secret(secret: &SecretValue) -> Option<String> {
    match secret {
        SecretValue::Inline { value } => {
            let v = value.expose_secret().to_string();
            (v != REDACTED_SECRET_SENTINEL).then_some(v)
        }
        SecretValue::FilePath { .. } => None,
    }
}

impl CredentialType {
    /// Merge redacted sentinel values from the existing credential.
    /// When the API response redacts secrets to "********" and the UI sends that back,
    /// this restores the original secret from the existing record.
    pub fn merge_redacted_secrets(&mut self, existing: &CredentialType) {
        match (self, existing) {
            (
                Self::SnmpV1 { community },
                Self::SnmpV1 {
                    community: existing_community,
                },
            )
            | (
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
                Self::SnmpV3 {
                    auth_password,
                    priv_password,
                    ..
                },
                Self::SnmpV3 {
                    auth_password: existing_auth,
                    priv_password: existing_priv,
                    ..
                },
            ) => {
                if auth_password.is_redacted_sentinel() {
                    *auth_password = existing_auth.clone();
                }
                if priv_password.is_redacted_sentinel() {
                    *priv_password = existing_priv.clone();
                }
            }
            (
                Self::SshPassword { password, .. },
                Self::SshPassword {
                    password: existing_password,
                    ..
                },
            ) => {
                if password.is_redacted_sentinel() {
                    *password = existing_password.clone();
                }
            }
            (
                Self::SshPrivateKey {
                    private_key,
                    passphrase,
                    ..
                },
                Self::SshPrivateKey {
                    private_key: existing_key,
                    passphrase: existing_passphrase,
                    ..
                },
            ) => {
                if private_key.is_redacted_sentinel() {
                    *private_key = existing_key.clone();
                }
                if passphrase
                    .as_ref()
                    .is_some_and(SecretValue::is_redacted_sentinel)
                {
                    *passphrase = existing_passphrase.clone();
                }
            }
            (
                Self::ActiveDirectoryLdaps { password, .. },
                Self::ActiveDirectoryLdaps {
                    password: existing_password,
                    ..
                },
            ) => {
                if password.is_redacted_sentinel() {
                    *password = existing_password.clone();
                }
            }
            (Self::ActiveDirectoryKerberos { .. }, Self::ActiveDirectoryKerberos { .. }) => {}
            (
                Self::DockerProxy { ssl_key, .. },
                Self::DockerProxy {
                    ssl_key: existing_key,
                    ..
                },
            )
            | (
                Self::PodmanProxy { ssl_key, .. },
                Self::PodmanProxy {
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
            (
                Self::UnifiApiKey { api_key, .. },
                Self::UnifiApiKey {
                    api_key: existing_key,
                    ..
                },
            ) => {
                if api_key.is_redacted_sentinel() {
                    *api_key = existing_key.clone();
                }
            }
            (
                Self::UnifiLocalAdmin { password, .. },
                Self::UnifiLocalAdmin {
                    password: existing_password,
                    ..
                },
            ) => {
                if password.is_redacted_sentinel() {
                    *password = existing_password.clone();
                }
            }
            (
                Self::WindowsLocalAccount { password, .. },
                Self::WindowsLocalAccount {
                    password: existing_password,
                    ..
                },
            ) => {
                if password.is_redacted_sentinel() {
                    *password = existing_password.clone();
                }
            }
            (
                Self::WindowsDomainAccount { password, .. },
                Self::WindowsDomainAccount {
                    password: existing_password,
                    ..
                },
            ) => {
                if password.is_redacted_sentinel() {
                    *password = existing_password.clone();
                }
            }
            // Every remaining arm is "nothing to merge": either the variant holds no secret, or
            // the credential's type was changed in this edit so there is no prior secret of the
            // same shape to restore.
            //
            // Enumerated rather than wildcarded on purpose. This match is the only thing standing
            // between a redacted round-trip and destroying a stored secret — a `_ => {}` lets a
            // newly added variant compile clean and silently persist the literal "********",
            // which then fails every probe with the real secret already gone.
            (Self::SnmpV1 { .. }, _)
            | (Self::SnmpV2c { .. }, _)
            | (Self::SnmpV3 { .. }, _)
            | (Self::SshPassword { .. }, _)
            | (Self::SshPrivateKey { .. }, _)
            | (Self::ActiveDirectoryLdaps { .. }, _)
            | (Self::ActiveDirectoryKerberos { .. }, _)
            | (Self::DockerProxy { .. }, _)
            | (Self::DockerSocket { .. }, _)
            | (Self::PodmanProxy { .. }, _)
            | (Self::PodmanSocket { .. }, _)
            | (Self::UnifiApiKey { .. }, _)
            | (Self::UnifiLocalAdmin { .. }, _)
            | (Self::WindowsLocalAccount { .. }, _)
            | (Self::WindowsDomainAccount { .. }, _) => {}
        }
    }

    pub fn credential_category(&self) -> CredentialCategory {
        match self {
            Self::SnmpV1 { .. } | Self::SnmpV2c { .. } | Self::SnmpV3 { .. } => {
                CredentialCategory::NetworkMonitoring
            }
            Self::SshPassword { .. } | Self::SshPrivateKey { .. } => {
                CredentialCategory::RemoteAccess
            }
            Self::ActiveDirectoryLdaps { .. } | Self::ActiveDirectoryKerberos { .. } => {
                CredentialCategory::IdentityAndAccess
            }
            Self::DockerProxy { .. }
            | Self::DockerSocket { .. }
            | Self::PodmanProxy { .. }
            | Self::PodmanSocket { .. } => CredentialCategory::ContainerVirtualization,
            Self::UnifiApiKey { .. } | Self::UnifiLocalAdmin { .. } => {
                CredentialCategory::NetworkController
            }
            Self::WindowsLocalAccount { .. } | Self::WindowsDomainAccount { .. } => {
                CredentialCategory::RemoteAccess
            }
        }
    }

    /// Where this credential type can be applied: the daemon's own host, specific
    /// hosts, and/or a whole network (broadcast).
    pub fn targets(&self) -> Vec<Target> {
        match self {
            // SNMP can target the daemon's own host too (a 127.0.0.1 IP-override),
            // specific hosts, or a whole network.
            Self::SnmpV1 { .. } | Self::SnmpV2c { .. } | Self::SnmpV3 { .. } => {
                vec![Target::DaemonHost, Target::Hosts, Target::Network]
            }
            Self::SshPassword { .. } | Self::SshPrivateKey { .. } => {
                vec![Target::DaemonHost, Target::Hosts, Target::Network]
            }
            Self::ActiveDirectoryLdaps { .. } | Self::ActiveDirectoryKerberos { .. } => {
                vec![Target::Hosts]
            }
            // Docker/Podman proxy: on the daemon host (localhost proxy) or remote hosts.
            Self::DockerProxy { .. } | Self::PodmanProxy { .. } => {
                vec![Target::DaemonHost, Target::Hosts]
            }
            // Local socket: only the daemon's own host.
            Self::DockerSocket { .. } | Self::PodmanSocket { .. } => vec![Target::DaemonHost],
            // A controller is one specific endpoint: the daemon's own host (self-hosted
            // controller) or a named host. Deliberately NOT `Network` — a network target is
            // broadcast as the default credential for every IP in the subnet, which would
            // spray controller credentials at unrelated hosts.
            Self::UnifiApiKey { .. } | Self::UnifiLocalAdmin { .. } => {
                vec![Target::DaemonHost, Target::Hosts]
            }
            Self::WindowsLocalAccount { .. } | Self::WindowsDomainAccount { .. } => {
                vec![Target::DaemonHost, Target::Hosts, Target::Network]
            }
        }
    }

    /// Whether the user must provide any configuration (fields) for this type.
    /// Derived from `field_definitions()` — a type with no fields needs nothing.
    pub fn requires_config(&self) -> bool {
        !self.field_definitions().is_empty()
    }

    /// Minimum daemon version that can safely receive this credential type over the
    /// wire. Delegates to the discriminant's exhaustive declaration
    /// ([`CredentialTypeDiscriminants::minimum_daemon_version`]).
    pub fn minimum_daemon_version(&self) -> semver::Version {
        CredentialTypeDiscriminants::from(self).minimum_daemon_version()
    }

    /// Whether this integration is a single service instance per host, so its
    /// access methods at a given target are mutually exclusive (e.g. a container
    /// runtime is reached by exactly one of socket/proxy). `false` for try-many
    /// auth integrations like SNMP (multiple credentials are attempted). All
    /// credential types of the same integration agree.
    pub fn single_endpoint_per_host(&self) -> bool {
        match self {
            Self::DockerProxy { .. }
            | Self::DockerSocket { .. }
            | Self::PodmanProxy { .. }
            | Self::PodmanSocket { .. }
            // One controller instance per host; API key and local admin are two ways in.
            | Self::UnifiApiKey { .. }
            | Self::UnifiLocalAdmin { .. } => true,
            Self::SnmpV1 { .. } | Self::SnmpV2c { .. } | Self::SnmpV3 { .. } => false,
            Self::SshPassword { .. }
            | Self::SshPrivateKey { .. }
            | Self::ActiveDirectoryLdaps { .. }
            | Self::ActiveDirectoryKerberos { .. } => false,
            Self::WindowsLocalAccount { .. } | Self::WindowsDomainAccount { .. } => false,
        }
    }

    /// Get the inline string value for a field by ID.
    /// Returns None for FilePath mode, None fields, or redacted sentinels.
    fn get_inline_value(&self, field_id: &str) -> Option<String> {
        match self {
            Self::SnmpV1 { community } | Self::SnmpV2c { community } => match field_id {
                "community" => inline_secret(community),
                _ => None,
            },
            Self::SnmpV3 {
                auth_password,
                priv_password,
                ..
            } => match field_id {
                "auth_password" => inline_secret(auth_password),
                "priv_password" => inline_secret(priv_password),
                _ => None,
            },
            Self::SshPassword { password, .. } => match field_id {
                "password" => inline_secret(password),
                _ => None,
            },
            Self::SshPrivateKey {
                private_key,
                passphrase,
                ..
            } => match field_id {
                "private_key" => inline_secret(private_key),
                "passphrase" => inline_secret(passphrase.as_ref()?),
                _ => None,
            },
            Self::ActiveDirectoryLdaps {
                password,
                ca_certificate,
                ..
            } => match field_id {
                "password" => inline_secret(password),
                "ca_certificate" => match ca_certificate.as_ref()? {
                    FileOrInline::Inline { value } => Some(value.clone()),
                    FileOrInline::FilePath { .. } => None,
                },
                _ => None,
            },
            Self::ActiveDirectoryKerberos { ca_certificate, .. } => match field_id {
                "ca_certificate" => match ca_certificate.as_ref()? {
                    FileOrInline::Inline { value } => Some(value.clone()),
                    FileOrInline::FilePath { .. } => None,
                },
                _ => None,
            },
            Self::DockerProxy {
                ssl_cert,
                ssl_key,
                ssl_chain,
                ..
            }
            | Self::PodmanProxy {
                ssl_cert,
                ssl_key,
                ssl_chain,
                ..
            } => match field_id {
                "ssl_cert" => match ssl_cert.as_ref()? {
                    FileOrInline::Inline { value } => Some(value.clone()),
                    FileOrInline::FilePath { .. } => None,
                },
                "ssl_key" => inline_secret(ssl_key.as_ref()?),
                "ssl_chain" => match ssl_chain.as_ref()? {
                    FileOrInline::Inline { value } => Some(value.clone()),
                    FileOrInline::FilePath { .. } => None,
                },
                _ => None,
            },
            Self::UnifiApiKey { api_key, .. } => match field_id {
                "api_key" => inline_secret(api_key),
                _ => None,
            },
            Self::UnifiLocalAdmin { password, .. } => match field_id {
                "password" => inline_secret(password),
                _ => None,
            },
            Self::DockerSocket { .. } | Self::PodmanSocket { .. } => None,
            Self::WindowsLocalAccount { password, .. }
            | Self::WindowsDomainAccount { password, .. } => match field_id {
                "password" => inline_secret(password),
                _ => None,
            },
        }
    }

    /// Validate inline field values using field_definitions() metadata.
    /// Skips FilePath values (validated on daemon after read), redacted sentinels,
    /// and empty optionals.
    pub fn validate(&self) -> Result<(), Error> {
        if let Self::SshPassword {
            username,
            port,
            host_key_policy,
            known_hosts_file,
            ..
        }
        | Self::SshPrivateKey {
            username,
            port,
            host_key_policy,
            known_hosts_file,
            ..
        } = self
        {
            if username.trim().is_empty() {
                crate::bail_validation!("SSH username cannot be empty");
            }
            if *port == 0 {
                crate::bail_validation!("SSH port must be between 1 and 65535");
            }
            if *host_key_policy == SshHostKeyPolicy::Strict
                && known_hosts_file.as_deref().is_none_or(str::is_empty)
            {
                crate::bail_validation!(
                    "Strict SSH host-key verification requires a known_hosts file"
                );
            }
            if let Some(path) = known_hosts_file
                && !is_supported_absolute_path(path)
            {
                crate::bail_validation!("SSH known_hosts file must use an absolute path");
            }
        }
        if let Self::WindowsLocalAccount { username, port, .. }
        | Self::WindowsDomainAccount { username, port, .. } = self
        {
            if username.trim().is_empty() {
                crate::bail_validation!("Windows account username cannot be empty");
            }
            if *port == 0 {
                crate::bail_validation!("WinRM port must be between 1 and 65535");
            }
        }
        if let Self::WindowsDomainAccount { domain, .. } = self
            && domain.trim().is_empty()
        {
            crate::bail_validation!("Windows domain account requires a non-empty domain");
        }
        if let Self::ActiveDirectoryLdaps {
            bind_dn,
            port,
            server_name,
            base_dn,
            group_dns,
            ..
        } = self
        {
            if !is_valid_ad_dn(bind_dn) {
                crate::bail_validation!(
                    "Active Directory bind DN must be a bounded distinguished name"
                );
            }
            if *port == 0 {
                crate::bail_validation!("LDAPS port must be between 1 and 65535");
            }
            if !is_valid_dns_name(server_name) {
                crate::bail_validation!("LDAPS server name must be a valid DNS name");
            }
            if !is_valid_ad_dn(base_dn) {
                crate::bail_validation!(
                    "Active Directory base DN must be a bounded distinguished name"
                );
            }
            if let Some(groups) = group_dns {
                let configured = groups
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<_>>();
                if groups.chars().count() > MAX_AD_GROUP_SCOPE_CHARS
                    || configured.len() > MAX_AD_GROUPS
                    || configured.iter().any(|group| !is_valid_ad_dn(group))
                {
                    crate::bail_validation!(
                        "Active Directory group scope exceeds its line, count, or total limit"
                    );
                }
            }
        }
        if let Self::ActiveDirectoryKerberos {
            principal,
            use_system_ccache,
            port,
            server_name,
            base_dn,
            group_dns,
            ..
        } = self
        {
            if !is_valid_kerberos_principal(principal) {
                crate::bail_validation!(
                    "Kerberos principal must be a bounded non-empty principal@REALM value"
                );
            }
            if !use_system_ccache {
                crate::bail_validation!(
                    "Kerberos credentials require explicit use of the daemon system ccache"
                );
            }
            if *port == 0 {
                crate::bail_validation!("LDAPS port must be between 1 and 65535");
            }
            if !is_valid_dns_name(server_name) {
                crate::bail_validation!("LDAPS server name must be a valid DNS name");
            }
            if !is_valid_ad_dn(base_dn) {
                crate::bail_validation!(
                    "Active Directory base DN must be a bounded distinguished name"
                );
            }
            if let Some(groups) = group_dns {
                let configured = groups
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<_>>();
                if groups.chars().count() > MAX_AD_GROUP_SCOPE_CHARS
                    || configured.len() > MAX_AD_GROUPS
                    || configured.iter().any(|group| !is_valid_ad_dn(group))
                {
                    crate::bail_validation!(
                        "Active Directory group scope exceeds its line, count, or total limit"
                    );
                }
            }
        }
        for field in self.field_definitions() {
            if let Some(fmt) = &field.inline_format
                && let Some(value) = self.get_inline_value(field.id)
            {
                fmt.validate(&value, field.label)?;
            }
        }
        Ok(())
    }

    /// Returns the ServiceDefinition this credential type integrates with.
    /// Every credential type maps to exactly one service — used for logo display,
    /// metadata enrichment, and Phase 2 integration dispatch.
    pub fn associated_service(&self) -> Box<dyn ServiceDefinition> {
        match self {
            Self::SnmpV1 { .. } | Self::SnmpV2c { .. } | Self::SnmpV3 { .. } => {
                Box::new(crate::server::services::definitions::snmp::Snmp)
            }
            Self::SshPassword { .. } | Self::SshPrivateKey { .. } => {
                Box::new(crate::server::services::definitions::ssh::Ssh)
            }
            Self::ActiveDirectoryLdaps { .. } | Self::ActiveDirectoryKerberos { .. } => {
                Box::new(crate::server::services::definitions::active_directory::ActiveDirectory)
            }
            Self::DockerProxy { .. } | Self::DockerSocket { .. } => {
                Box::new(crate::server::services::definitions::docker_daemon::Docker)
            }
            Self::PodmanProxy { .. } | Self::PodmanSocket { .. } => {
                Box::new(crate::server::services::definitions::podman::Podman)
            }
            Self::UnifiApiKey { .. } | Self::UnifiLocalAdmin { .. } => {
                Box::new(crate::server::services::definitions::unifi_controller::UnifiController)
            }
            Self::WindowsLocalAccount { .. } | Self::WindowsDomainAccount { .. } => {
                Box::new(crate::server::services::definitions::windows::Windows)
            }
        }
    }

    /// Convert to wire format payload for daemon transmission.
    /// No wildcard match — compiler forces update when new variants added.
    pub fn to_query_payload(&self) -> CredentialQueryPayload {
        use crate::server::credentials::r#impl::mapping::{SnmpQueryCredential, SnmpV3Params};
        match self {
            CredentialType::SnmpV1 { community } => {
                CredentialQueryPayload::Snmp(SnmpQueryCredential {
                    version: SnmpVersion::V1,
                    community: secret_to_resolvable(community),
                    v3: None,
                })
            }
            CredentialType::SnmpV2c { community } => {
                CredentialQueryPayload::Snmp(SnmpQueryCredential {
                    version: SnmpVersion::V2c,
                    community: secret_to_resolvable(community),
                    v3: None,
                })
            }
            CredentialType::SnmpV3 {
                security_name,
                auth_protocol,
                auth_password,
                priv_protocol,
                priv_password,
                context_name,
            } => CredentialQueryPayload::Snmp(SnmpQueryCredential {
                version: SnmpVersion::V3,
                community: ResolvableSecret::Value {
                    value: String::new(),
                },
                v3: Some(SnmpV3Params {
                    security_name: security_name.clone(),
                    auth_protocol: *auth_protocol,
                    auth_password: secret_to_resolvable(auth_password),
                    priv_protocol: *priv_protocol,
                    priv_password: secret_to_resolvable(priv_password),
                    context_name: context_name.clone(),
                }),
            }),
            CredentialType::SshPassword {
                username,
                password,
                port,
                platform,
                host_key_policy,
                known_hosts_file,
            } => CredentialQueryPayload::Ssh(SshQueryCredential {
                username: username.clone(),
                authentication: SshAuthentication::Password {
                    password: secret_to_resolvable(password),
                },
                port: *port,
                platform: *platform,
                host_key_policy: *host_key_policy,
                known_hosts_file: known_hosts_file.clone(),
            }),
            CredentialType::SshPrivateKey {
                username,
                private_key,
                passphrase,
                port,
                platform,
                host_key_policy,
                known_hosts_file,
            } => CredentialQueryPayload::Ssh(SshQueryCredential {
                username: username.clone(),
                authentication: SshAuthentication::PrivateKey {
                    private_key: secret_to_resolvable(private_key),
                    passphrase: passphrase.as_ref().map(secret_to_resolvable),
                },
                port: *port,
                platform: *platform,
                host_key_policy: *host_key_policy,
                known_hosts_file: known_hosts_file.clone(),
            }),
            CredentialType::ActiveDirectoryLdaps {
                bind_dn,
                password,
                port,
                server_name,
                base_dn,
                ca_certificate,
                group_dns,
            } => {
                CredentialQueryPayload::ActiveDirectoryLdaps(ActiveDirectoryLdapsQueryCredential {
                    bind_dn: bind_dn.clone(),
                    password: secret_to_resolvable(password),
                    port: *port,
                    server_name: server_name.clone(),
                    base_dn: base_dn.clone(),
                    ca_certificate: ca_certificate.as_ref().map(|value| match value {
                        FileOrInline::Inline { value } => ResolvableValue::Value {
                            value: value.clone(),
                        },
                        FileOrInline::FilePath { path } => {
                            ResolvableValue::FilePath { path: path.clone() }
                        }
                    }),
                    group_dns: group_dns.clone(),
                })
            }
            CredentialType::ActiveDirectoryKerberos {
                principal,
                use_system_ccache,
                port,
                server_name,
                base_dn,
                ca_certificate,
                group_dns,
            } => CredentialQueryPayload::ActiveDirectoryKerberos(
                ActiveDirectoryKerberosQueryCredential {
                    principal: principal.clone(),
                    use_system_ccache: *use_system_ccache,
                    port: *port,
                    server_name: server_name.clone(),
                    base_dn: base_dn.clone(),
                    ca_certificate: ca_certificate.as_ref().map(|value| match value {
                        FileOrInline::Inline { value } => ResolvableValue::Value {
                            value: value.clone(),
                        },
                        FileOrInline::FilePath { path } => {
                            ResolvableValue::FilePath { path: path.clone() }
                        }
                    }),
                    group_dns: group_dns.clone(),
                },
            ),
            CredentialType::DockerProxy {
                port,
                path,
                ssl_cert,
                ssl_key,
                ssl_chain,
            } => CredentialQueryPayload::DockerProxy(container_proxy_query(
                *port, path, ssl_cert, ssl_key, ssl_chain,
            )),
            CredentialType::DockerSocket { socket_path } => {
                CredentialQueryPayload::DockerSocket(ContainerSocketQueryCredential {
                    socket_path: socket_path.clone(),
                })
            }
            CredentialType::PodmanProxy {
                port,
                path,
                ssl_cert,
                ssl_key,
                ssl_chain,
            } => CredentialQueryPayload::PodmanProxy(container_proxy_query(
                *port, path, ssl_cert, ssl_key, ssl_chain,
            )),
            CredentialType::PodmanSocket { socket_path } => {
                CredentialQueryPayload::PodmanSocket(ContainerSocketQueryCredential {
                    socket_path: socket_path.clone(),
                })
            }
            // Both UniFi transports collapse to one wire payload — same endpoint, same site,
            // only the auth material differs.
            CredentialType::UnifiApiKey {
                port,
                site,
                api_key,
            } => CredentialQueryPayload::UnifiController(UnifiQueryCredential {
                port: *port,
                site: site.clone(),
                auth: UnifiAuth::ApiKey {
                    api_key: secret_to_resolvable(api_key),
                },
            }),
            CredentialType::UnifiLocalAdmin {
                port,
                site,
                username,
                password,
            } => CredentialQueryPayload::UnifiController(UnifiQueryCredential {
                port: *port,
                site: site.clone(),
                auth: UnifiAuth::LocalAdmin {
                    username: username.clone(),
                    password: secret_to_resolvable(password),
                },
            }),
            CredentialType::WindowsLocalAccount {
                username,
                password,
                port,
                use_tls,
                accept_invalid_certs,
            } => CredentialQueryPayload::WindowsLocalAccount(WindowsLocalAccountQueryCredential {
                username: username.clone(),
                password: secret_to_resolvable(password),
                port: *port,
                use_tls: *use_tls,
                accept_invalid_certs: *accept_invalid_certs,
            }),
            CredentialType::WindowsDomainAccount {
                domain,
                username,
                password,
                port,
                use_tls,
                accept_invalid_certs,
            } => {
                CredentialQueryPayload::WindowsDomainAccount(WindowsDomainAccountQueryCredential {
                    domain: domain.clone(),
                    username: username.clone(),
                    password: secret_to_resolvable(password),
                    port: *port,
                    use_tls: *use_tls,
                    accept_invalid_certs: *accept_invalid_certs,
                })
            }
        }
    }
}

/// Build a container-runtime proxy query credential from the shared
/// proxy fields (Docker and Podman use the same Docker-compatible shape).
fn container_proxy_query(
    port: u16,
    path: &Option<String>,
    ssl_cert: &Option<FileOrInline>,
    ssl_key: &Option<SecretValue>,
    ssl_chain: &Option<FileOrInline>,
) -> ContainerProxyQueryCredential {
    let file_or_inline = |f: &FileOrInline| match f {
        FileOrInline::Inline { value } => ResolvableValue::Value {
            value: value.clone(),
        },
        FileOrInline::FilePath { path } => ResolvableValue::FilePath { path: path.clone() },
    };
    ContainerProxyQueryCredential {
        port,
        path: path.clone(),
        ssl_cert: ssl_cert.as_ref().map(&file_or_inline),
        ssl_key: ssl_key.as_ref().map(secret_to_resolvable),
        ssl_chain: ssl_chain.as_ref().map(&file_or_inline),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::credentials::r#impl::mapping::CredentialQueryPayload;
    use secrecy::SecretString;
    use std::collections::HashMap;
    use strum::IntoEnumIterator;

    /// `single_endpoint_per_host()` is an integration-level property expressed
    /// per credential type, so every credential type sharing an `associated_service`
    /// must agree — otherwise the integration's exclusivity rule is incoherent.
    #[test]
    fn single_endpoint_per_host_agrees_within_integration() {
        let mut by_service: HashMap<&'static str, bool> = HashMap::new();
        for disc in CredentialTypeDiscriminants::iter() {
            let ct = disc.to_credential_type();
            let service = ct.associated_service().name();
            let value = ct.single_endpoint_per_host();
            match by_service.get(service) {
                Some(&existing) => assert_eq!(
                    existing, value,
                    "credential types for integration '{service}' disagree on single_endpoint_per_host",
                ),
                None => {
                    by_service.insert(service, value);
                }
            }
        }
    }

    fn snmp_cred(community: &str) -> CredentialType {
        CredentialType::SnmpV2c {
            community: SecretValue::Inline {
                value: SecretString::from(community.to_string()),
            },
        }
    }

    fn inline(value: &str) -> SecretValue {
        SecretValue::Inline {
            value: SecretString::from(value.to_string()),
        }
    }

    fn ssh_password_cred(known_hosts_file: Option<&str>) -> CredentialType {
        CredentialType::SshPassword {
            username: "scanopy".to_string(),
            password: inline("secret"),
            port: 22,
            platform: SshPlatform::Linux,
            host_key_policy: SshHostKeyPolicy::Strict,
            known_hosts_file: known_hosts_file.map(str::to_string),
        }
    }

    fn ad_ldaps_cred(password: &str) -> CredentialType {
        CredentialType::ActiveDirectoryLdaps {
            bind_dn: "CN=Scanopy,DC=example,DC=com".to_string(),
            password: inline(password),
            port: 636,
            server_name: "dc01.example.com".to_string(),
            base_dn: "DC=example,DC=com".to_string(),
            ca_certificate: None,
            group_dns: None,
        }
    }

    #[test]
    fn ad_ldaps_is_host_only_and_preserves_redacted_password() {
        let existing = ad_ldaps_cred("directory-secret");
        let mut updated = ad_ldaps_cred(REDACTED_SECRET_SENTINEL);
        updated.merge_redacted_secrets(&existing);

        assert_eq!(updated.targets(), vec![Target::Hosts]);
        assert_eq!(updated, existing);
        assert!(updated.validate().is_ok());
    }

    #[test]
    fn ad_kerberos_requires_system_cache_acknowledgement_and_daemon_feature() {
        let mut credential = CredentialType::ActiveDirectoryKerberos {
            principal: "scanopy-reader@EXAMPLE.COM".to_string(),
            use_system_ccache: true,
            port: 636,
            server_name: "dc01.example.com".to_string(),
            base_dn: "DC=example,DC=com".to_string(),
            ca_certificate: None,
            group_dns: None,
        };
        assert!(credential.validate().is_ok());
        let disc = CredentialTypeDiscriminants::from(&credential);
        let version = semver::Version::new(0, 19, 0);
        assert!(!disc.compatible_with_daemon(Some(&version)));
        assert!(!disc.compatible_with_daemon_features(Some(&version), &[]));
        assert!(disc.compatible_with_daemon_features(
            Some(&version),
            &[crate::server::daemons::r#impl::base::ACTIVE_DIRECTORY_GSSAPI_FEATURE.to_string()]
        ));
        assert_eq!(
            disc.required_daemon_features(),
            vec![crate::server::daemons::r#impl::base::ACTIVE_DIRECTORY_GSSAPI_FEATURE]
        );

        if let CredentialType::ActiveDirectoryKerberos {
            use_system_ccache, ..
        } = &mut credential
        {
            *use_system_ccache = false;
        }
        assert!(credential.validate().is_err());
    }


    #[test]
    fn ssh_known_hosts_validation_accepts_daemon_platform_absolute_paths() {
        assert!(
            ssh_password_cred(Some("/root/.ssh/known_hosts"))
                .validate()
                .is_ok()
        );
        assert!(
            ssh_password_cred(Some("C:\\ssh\\known_hosts"))
                .validate()
                .is_ok()
        );
        assert!(
            ssh_password_cred(Some("\\\\server\\share\\known_hosts"))
                .validate()
                .is_ok()
        );
        assert!(
            ssh_password_cred(Some("relative/known_hosts"))
                .validate()
                .is_err()
        );
    }

    fn snmpv1_cred(community: &str) -> CredentialType {
        CredentialType::SnmpV1 {
            community: inline(community),
        }
    }

    fn snmpv3_cred(auth_pw: &str, priv_pw: &str) -> CredentialType {
        CredentialType::SnmpV3 {
            security_name: "snmp-user".to_string(),
            auth_protocol: SnmpV3AuthProtocol::Sha256,
            auth_password: inline(auth_pw),
            priv_protocol: SnmpV3PrivProtocol::Aes128,
            priv_password: inline(priv_pw),
            context_name: Some("ctx-1".to_string()),
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

    /// Both UniFi transports carry a secret, and the redacted round-trip is the one thing the
    /// compiler cannot check for them: without an arm here the edit would silently persist the
    /// literal "********" and destroy the stored credential.
    #[test]
    fn merge_redacted_secrets_preserves_both_unifi_transports() {
        let mut updated = CredentialType::UnifiApiKey {
            port: 443,
            site: "default".to_string(),
            api_key: SecretValue::Inline {
                value: SecretString::from(REDACTED_SECRET_SENTINEL.to_string()),
            },
        };
        updated.merge_redacted_secrets(&CredentialType::UnifiApiKey {
            port: 443,
            site: "default".to_string(),
            api_key: SecretValue::Inline {
                value: SecretString::from("real-api-key".to_string()),
            },
        });
        match &updated {
            CredentialType::UnifiApiKey {
                api_key: SecretValue::Inline { value },
                ..
            } => assert_eq!(value.expose_secret(), "real-api-key"),
            _ => panic!("Expected UnifiApiKey with an inline key"),
        }

        let mut updated = CredentialType::UnifiLocalAdmin {
            port: 8443,
            site: "default".to_string(),
            username: "scanopy".to_string(),
            password: SecretValue::Inline {
                value: SecretString::from(REDACTED_SECRET_SENTINEL.to_string()),
            },
        };
        updated.merge_redacted_secrets(&CredentialType::UnifiLocalAdmin {
            port: 8443,
            site: "default".to_string(),
            username: "scanopy".to_string(),
            password: SecretValue::Inline {
                value: SecretString::from("real-password".to_string()),
            },
        });
        match &updated {
            CredentialType::UnifiLocalAdmin {
                password: SecretValue::Inline { value },
                ..
            } => assert_eq!(value.expose_secret(), "real-password"),
            _ => panic!("Expected UnifiLocalAdmin with an inline password"),
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

    #[test]
    fn merge_redacted_secrets_snmpv1_preserves_original() {
        let existing = snmpv1_cred("legacy-community");
        let mut updated = snmpv1_cred(REDACTED_SECRET_SENTINEL);
        updated.merge_redacted_secrets(&existing);
        match &updated {
            CredentialType::SnmpV1 {
                community: SecretValue::Inline { value },
            } => assert_eq!(value.expose_secret(), "legacy-community"),
            _ => panic!("expected SnmpV1 inline"),
        }
    }

    #[test]
    fn merge_redacted_secrets_snmpv3_preserves_both_passwords_when_sentinel() {
        let existing = snmpv3_cred("auth-real", "priv-real");
        let mut updated = snmpv3_cred(REDACTED_SECRET_SENTINEL, REDACTED_SECRET_SENTINEL);
        updated.merge_redacted_secrets(&existing);
        match &updated {
            CredentialType::SnmpV3 {
                auth_password: SecretValue::Inline { value: a },
                priv_password: SecretValue::Inline { value: p },
                ..
            } => {
                assert_eq!(a.expose_secret(), "auth-real");
                assert_eq!(p.expose_secret(), "priv-real");
            }
            _ => panic!("expected SnmpV3 inline passwords"),
        }
    }

    #[test]
    fn merge_redacted_secrets_snmpv3_independent_per_password() {
        // Only the priv password is the sentinel — auth must keep its new value.
        let existing = snmpv3_cred("auth-real", "priv-real");
        let mut updated = snmpv3_cred("auth-new", REDACTED_SECRET_SENTINEL);
        updated.merge_redacted_secrets(&existing);
        match &updated {
            CredentialType::SnmpV3 {
                auth_password: SecretValue::Inline { value: a },
                priv_password: SecretValue::Inline { value: p },
                ..
            } => {
                assert_eq!(a.expose_secret(), "auth-new");
                assert_eq!(p.expose_secret(), "priv-real");
            }
            _ => panic!("expected SnmpV3 inline passwords"),
        }
    }

    #[test]
    fn to_query_payload_snmpv1_sets_version_and_community() {
        match snmpv1_cred("comm").to_query_payload() {
            CredentialQueryPayload::Snmp(s) => {
                assert_eq!(s.version, SnmpVersion::V1);
                assert!(s.v3.is_none());
                assert_eq!(
                    s.community,
                    ResolvableSecret::Value {
                        value: "comm".to_string()
                    }
                );
            }
            _ => panic!("expected Snmp payload"),
        }
    }

    #[test]
    fn to_query_payload_snmpv3_populates_usm_params() {
        match snmpv3_cred("auth-pw", "priv-pw").to_query_payload() {
            CredentialQueryPayload::Snmp(s) => {
                assert_eq!(s.version, SnmpVersion::V3);
                let v3 = s.v3.expect("v3 params present");
                assert_eq!(v3.security_name, "snmp-user");
                assert_eq!(v3.auth_protocol, SnmpV3AuthProtocol::Sha256);
                assert_eq!(v3.priv_protocol, SnmpV3PrivProtocol::Aes128);
                assert_eq!(v3.context_name.as_deref(), Some("ctx-1"));
                assert_eq!(
                    v3.auth_password,
                    ResolvableSecret::Value {
                        value: "auth-pw".to_string()
                    }
                );
                assert_eq!(
                    v3.priv_password,
                    ResolvableSecret::Value {
                        value: "priv-pw".to_string()
                    }
                );
            }
            _ => panic!("expected Snmp payload"),
        }
    }

    #[test]
    fn snmpv3_default_serialization_redacts_passwords() {
        // The default Serialize impl (API responses) must never leak secrets.
        let json = serde_json::to_string(&snmpv3_cred("super-secret-auth", "super-secret-priv"))
            .expect("serialize");
        assert!(!json.contains("super-secret-auth"));
        assert!(!json.contains("super-secret-priv"));
        assert!(json.contains(REDACTED_SECRET_SENTINEL));
    }

    #[test]
    fn snmpv3_storage_serialization_exposes_passwords() {
        // Under an ExposeSecretsGuard (the DB-write path), secrets are exposed.
        let cred = snmpv3_cred("auth-xyz-plain", "priv-xyz-plain");
        let json = {
            let _expose = ExposeSecretsGuard::new();
            serde_json::to_string(&cred).expect("serialize")
        };
        assert!(json.contains("auth-xyz-plain"));
        assert!(json.contains("priv-xyz-plain"));
    }

    #[test]
    fn secret_exposure_resets_after_guard_drops() {
        // After the guard scope, serialization must redact again — the storage
        // exposure must not leak into subsequent default serializations.
        let cred = snmpv3_cred("leak-check-auth", "leak-check-priv");
        {
            let _expose = ExposeSecretsGuard::new();
            let _ = serde_json::to_string(&cred);
        }
        let json = serde_json::to_string(&cred).expect("serialize");
        assert!(!json.contains("leak-check-auth"));
        assert!(json.contains(REDACTED_SECRET_SENTINEL));
    }

    #[test]
    fn snmpv3_query_payload_debug_redacts_passwords() {
        let payload = snmpv3_cred("auth-dbg", "priv-dbg").to_query_payload();
        let dbg = format!("{:?}", payload);
        assert!(!dbg.contains("auth-dbg"));
        assert!(!dbg.contains("priv-dbg"));
    }

    fn podman_cred(ssl_key: Option<&str>) -> CredentialType {
        CredentialType::PodmanProxy {
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
    fn podman_proxy_to_query_payload_round_trips() {
        match podman_cred(Some("podman-key")).to_query_payload() {
            CredentialQueryPayload::PodmanProxy(p) => {
                assert_eq!(p.port, 2376);
                assert!(matches!(
                    p.ssl_key,
                    Some(ResolvableSecret::Value { value }) if value == "podman-key"
                ));
            }
            other => panic!("expected PodmanProxy payload, got {other:?}"),
        }
    }

    #[test]
    fn podman_socket_to_query_payload() {
        assert!(matches!(
            CredentialType::PodmanSocket { socket_path: None }.to_query_payload(),
            CredentialQueryPayload::PodmanSocket(_)
        ));
    }

    #[test]
    fn podman_credentials_are_container_virtualization() {
        assert_eq!(
            podman_cred(None).credential_category(),
            CredentialCategory::ContainerVirtualization
        );
        assert_eq!(
            CredentialType::PodmanSocket { socket_path: None }.credential_category(),
            CredentialCategory::ContainerVirtualization
        );
    }

    #[test]
    fn podman_proxy_associates_podman_service() {
        assert_eq!(podman_cred(None).associated_service().name(), "Podman");
        assert_eq!(
            CredentialType::PodmanSocket { socket_path: None }
                .associated_service()
                .name(),
            "Podman"
        );
    }

    #[test]
    fn podman_proxy_exposes_proxy_fields() {
        // Same shape as Docker proxy: port, path, and three TLS fields.
        let fields = podman_cred(None).field_definitions();
        let ids: Vec<&str> = fields.iter().map(|f| f.id).collect();
        assert_eq!(
            ids,
            vec!["port", "path", "ssl_cert", "ssl_key", "ssl_chain"]
        );
        // Socket types expose a single optional, non-secret socket_path field (repointable) and
        // target only the daemon host — the property the UI derives in place of the old
        // `is_local_auto` flag.
        let socket_fields = CredentialType::PodmanSocket { socket_path: None }.field_definitions();
        assert_eq!(socket_fields.len(), 1);
        assert_eq!(socket_fields[0].id, "socket_path");
        assert!(socket_fields[0].optional && !socket_fields[0].secret);
        assert_eq!(
            CredentialType::PodmanSocket { socket_path: None }.targets(),
            vec![Target::DaemonHost]
        );
    }

    #[test]
    fn storage_serialization_tag_round_trips_for_every_variant() {
        // The stored form is the credential's own derived serialization (under an
        // exposure guard). This guards that every variant's "type" tag round-trips
        // through Deserialize — so storage can't silently desync from how it reads.
        for disc in CredentialTypeDiscriminants::iter() {
            let ct = disc.to_credential_type();
            let json = {
                let _expose = ExposeSecretsGuard::new();
                serde_json::to_string(&ct).expect("serialize")
            };
            let back: CredentialType = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(
                CredentialTypeDiscriminants::from(&back),
                disc,
                "storage tag did not round-trip for {disc:?}: {json}"
            );
        }
    }

    #[test]
    fn podman_proxy_merge_redacted_ssl_key() {
        let existing = podman_cred(Some("original-key"));
        let mut updated = podman_cred(Some(REDACTED_SECRET_SENTINEL));
        updated.merge_redacted_secrets(&existing);
        match &updated {
            CredentialType::PodmanProxy { ssl_key, .. } => {
                assert!(matches!(
                    ssl_key,
                    Some(SecretValue::Inline { value }) if value.expose_secret() == "original-key"
                ));
            }
            _ => panic!("expected PodmanProxy variant"),
        }
    }

    /// The SNMP-sim seed script (`backend/scripts/seed-snmp-credentials.sql`, run by
    /// `make snmp-seed-credentials`) writes `credential_type` JSONB straight into the table,
    /// bypassing the API and therefore bypassing serde. Nothing else checks that what it writes
    /// is a shape the server can read back — a rename or a retagged field would leave rows that
    /// only fail at credential-load time, as a hand-written credential with a bare `community`
    /// string already did in the field (GH #611). Parsing the literals out of the script keeps
    /// this honest: the assertion is against the file the seed actually runs, not a copy of it.
    #[test]
    fn seeded_snmp_credential_json_deserializes() {
        let sql = include_str!("../../../../../scripts/seed-snmp-credentials.sql");

        // The JSONB literals are the single-quoted strings that open with `{"type":`.
        let literals: Vec<&str> = sql
            .split('\'')
            .filter(|chunk| chunk.trim_start().starts_with("{\"type\":"))
            .collect();

        assert_eq!(
            literals.len(),
            5,
            "expected the five sim credentials; found {} — did the script change shape?",
            literals.len()
        );

        let mut seen = std::collections::HashSet::new();
        for literal in &literals {
            let parsed: CredentialType = serde_json::from_str(literal).unwrap_or_else(|e| {
                panic!("seeded credential is not a CredentialType: {e}\n{literal}")
            });
            seen.insert(CredentialTypeDiscriminants::from(&parsed));
        }

        // The sim deliberately spreads devices across all three SNMP versions so a scan
        // exercises each negotiation path; losing one silently narrows what the env tests.
        for expected in [
            CredentialTypeDiscriminants::SnmpV1,
            CredentialTypeDiscriminants::SnmpV2c,
            CredentialTypeDiscriminants::SnmpV3,
        ] {
            assert!(
                seen.contains(&expected),
                "sim seed no longer covers {expected:?}"
            );
        }
    }
}

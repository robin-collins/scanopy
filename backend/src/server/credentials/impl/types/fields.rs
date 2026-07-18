use anyhow::Error;
use serde::Serialize;
use utoipa::ToSchema;

use super::CredentialType;

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
    /// Options for `Select` fields. Each option carries a wire `value` (the
    /// serialized enum variant) and a human-facing `label`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<&'static [SelectOption]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<&'static str>,
    /// For SecretPathOrInline fields: what format the inline value should be
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_format: Option<InlineFormat>,
    /// Optional group name for visually grouping fields in the UI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<&'static str>,
}

/// A single choice for a `Select` field. `value` is the wire value (serialized
/// enum variant, e.g. "Sha256"); `label` is the human-facing display text.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct SelectOption {
    pub value: &'static str,
    pub label: &'static str,
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
    OpenSshPrivateKey,
}

impl PemTag {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Certificate => "CERTIFICATE",
            Self::PrivateKey => "PRIVATE KEY",
            Self::RsaPrivateKey => "RSA PRIVATE KEY",
            Self::EcPrivateKey => "EC PRIVATE KEY",
            Self::OpenSshPrivateKey => "OPENSSH PRIVATE KEY",
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
                PemTag::OpenSshPrivateKey,
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
    Boolean,
    SecretPathOrInline,
    PathOrInline,
}

impl CredentialType {
    /// Returns field definitions for this credential type.
    /// Uses exhaustive destructuring for compile-time enforcement:
    /// adding a field to the enum variant without updating this method causes a compile error.
    pub fn field_definitions(&self) -> Vec<FieldDefinition> {
        match self {
            Self::SnmpV1 { community: _ } | Self::SnmpV2c { community: _ } => {
                vec![FieldDefinition {
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
                }]
            }
            Self::SnmpV3 {
                security_name: _,
                auth_protocol: _,
                auth_password: _,
                priv_protocol: _,
                priv_password: _,
                context_name: _,
            } => vec![
                FieldDefinition {
                    id: "security_name",
                    label: "Security Name",
                    field_type: FieldType::String,
                    placeholder: Some("snmp-user"),
                    secret: false,
                    optional: false,
                    help_text: Some("SNMPv3 USM user name (security name)."),
                    options: None,
                    default_value: None,
                    inline_format: None,
                    group: Some("Authentication"),
                },
                FieldDefinition {
                    id: "auth_protocol",
                    label: "Auth Protocol",
                    field_type: FieldType::Select,
                    placeholder: None,
                    secret: false,
                    optional: false,
                    help_text: Some("Authentication hash algorithm."),
                    options: Some(super::snmp::SnmpV3AuthProtocol::OPTIONS),
                    default_value: Some("Sha256"),
                    inline_format: None,
                    group: Some("Authentication"),
                },
                FieldDefinition {
                    id: "auth_password",
                    label: "Auth Password",
                    field_type: FieldType::SecretPathOrInline,
                    placeholder: None,
                    secret: true,
                    optional: false,
                    help_text: Some("Authentication password (minimum 8 characters)."),
                    options: None,
                    default_value: None,
                    inline_format: Some(InlineFormat::Plain),
                    group: Some("Authentication"),
                },
                FieldDefinition {
                    id: "priv_protocol",
                    label: "Privacy Protocol",
                    field_type: FieldType::Select,
                    placeholder: None,
                    secret: false,
                    optional: false,
                    help_text: Some("Privacy (encryption) algorithm."),
                    options: Some(super::snmp::SnmpV3PrivProtocol::OPTIONS),
                    default_value: Some("Aes128"),
                    inline_format: None,
                    group: Some("Privacy"),
                },
                FieldDefinition {
                    id: "priv_password",
                    label: "Privacy Password",
                    field_type: FieldType::SecretPathOrInline,
                    placeholder: None,
                    secret: true,
                    optional: false,
                    help_text: Some("Privacy (encryption) password (minimum 8 characters)."),
                    options: None,
                    default_value: None,
                    inline_format: Some(InlineFormat::Plain),
                    group: Some("Privacy"),
                },
                FieldDefinition {
                    id: "context_name",
                    label: "Context Name",
                    field_type: FieldType::String,
                    placeholder: Some("(default)"),
                    secret: false,
                    optional: true,
                    help_text: Some(
                        "Optional SNMPv3 context name. Leave blank for the default context (used by interface and LLDP data).",
                    ),
                    options: None,
                    default_value: None,
                    inline_format: None,
                    group: None,
                },
            ],
            Self::SshPassword { .. } => {
                let mut fields = ssh_connection_field_definitions();
                fields.insert(
                    1,
                    FieldDefinition {
                        id: "password",
                        label: "Password",
                        field_type: FieldType::SecretPathOrInline,
                        placeholder: None,
                        secret: true,
                        optional: false,
                        help_text: Some("Password for the read-only SSH account."),
                        options: None,
                        default_value: None,
                        inline_format: Some(InlineFormat::Plain),
                        group: Some("Authentication"),
                    },
                );
                fields
            }
            Self::SshPrivateKey { .. } => {
                let mut fields = ssh_connection_field_definitions();
                fields.insert(
                    1,
                    FieldDefinition {
                        id: "private_key",
                        label: "Private Key",
                        field_type: FieldType::SecretPathOrInline,
                        placeholder: Some("-----BEGIN OPENSSH PRIVATE KEY-----"),
                        secret: true,
                        optional: false,
                        help_text: Some(
                            "OpenSSH or PEM private key, inline or from a daemon-local file.",
                        ),
                        options: None,
                        default_value: None,
                        inline_format: Some(InlineFormat::PemPrivateKey),
                        group: Some("Authentication"),
                    },
                );
                fields.insert(
                    2,
                    FieldDefinition {
                        id: "passphrase",
                        label: "Key Passphrase",
                        field_type: FieldType::SecretPathOrInline,
                        placeholder: None,
                        secret: true,
                        optional: true,
                        help_text: Some("Passphrase for an encrypted private key, if required."),
                        options: None,
                        default_value: None,
                        inline_format: Some(InlineFormat::Plain),
                        group: Some("Authentication"),
                    },
                );
                fields
            }
            Self::ActiveDirectoryLdaps { .. } => active_directory_ldaps_field_definitions(),
            Self::ActiveDirectoryKerberos { .. } => active_directory_kerberos_field_definitions(),
            Self::UnifiPassword { .. } => unifi_password_field_definitions(),
            Self::DockerProxy { .. } => container_proxy_field_definitions(
                "Docker API Port",
                "Docker API port. Use 2375 for non-TLS proxies (Tecnativa, HAProxy) or 2376 for TLS.",
            ),
            Self::PodmanProxy { .. } => container_proxy_field_definitions(
                "Podman API Port",
                "Podman API port. Point at a TCP-exposed Podman service (e.g. `podman system service tcp:`) directly or behind a TLS proxy.",
            ),
            Self::DockerSocket { .. } => vec![socket_path_field(
                "/var/run/docker.sock",
                "Path to the Docker Unix socket. Leave blank to auto-detect (DOCKER_HOST or /var/run/docker.sock).",
            )],
            Self::PodmanSocket { .. } => vec![socket_path_field(
                "/run/podman/podman.sock",
                "Path to the Podman Unix socket. Leave blank to auto-detect (rootful /run/podman/podman.sock or the rootless $XDG_RUNTIME_DIR/podman/podman.sock).",
            )],
        }
    }
}

fn unifi_password_field_definitions() -> Vec<FieldDefinition> {
    vec![
        FieldDefinition {
            id: "controller_url",
            label: "Controller URL",
            field_type: FieldType::String,
            placeholder: Some("https://unifi.example.com:8443"),
            secret: false,
            optional: false,
            help_text: Some("HTTPS origin only. Requests are pinned to the assigned host IP."),
            options: None,
            default_value: None,
            inline_format: None,
            group: Some("Connection"),
        },
        FieldDefinition {
            id: "server_name",
            label: "TLS Server Name",
            field_type: FieldType::String,
            placeholder: Some("unifi.example.com"),
            secret: false,
            optional: false,
            help_text: Some("DNS identity verified by TLS; must match the controller URL host."),
            options: None,
            default_value: None,
            inline_format: None,
            group: Some("Connection"),
        },
        FieldDefinition {
            id: "site",
            label: "Site",
            field_type: FieldType::String,
            placeholder: Some("default"),
            secret: false,
            optional: false,
            help_text: Some("UniFi site identifier. Collection never expands to other sites."),
            options: None,
            default_value: Some("default"),
            inline_format: None,
            group: Some("Collection"),
        },
        FieldDefinition {
            id: "api_type",
            label: "Controller API",
            field_type: FieldType::Select,
            placeholder: None,
            secret: false,
            optional: false,
            help_text: Some(
                "UniFi OS uses the modern proxied API; older controllers use legacy paths.",
            ),
            options: Some(super::UnifiApiType::OPTIONS),
            default_value: Some("Modern"),
            inline_format: None,
            group: Some("Connection"),
        },
        FieldDefinition {
            id: "tls_policy",
            label: "TLS Policy",
            field_type: FieldType::Select,
            placeholder: None,
            secret: false,
            optional: false,
            help_text: Some(
                "Verification is the default. The exception applies only to this credential endpoint.",
            ),
            options: Some(super::UnifiTlsPolicy::OPTIONS),
            default_value: Some("Verify"),
            inline_format: None,
            group: Some("TLS"),
        },
        FieldDefinition {
            id: "username",
            label: "Username",
            field_type: FieldType::String,
            placeholder: Some("scanopy-readonly"),
            secret: false,
            optional: false,
            help_text: Some("Use a least-privilege local controller account without MFA."),
            options: None,
            default_value: None,
            inline_format: None,
            group: Some("Authentication"),
        },
        FieldDefinition {
            id: "password",
            label: "Password",
            field_type: FieldType::SecretPathOrInline,
            placeholder: None,
            secret: true,
            optional: false,
            help_text: Some("Password for the read-only local controller account."),
            options: None,
            default_value: None,
            inline_format: Some(InlineFormat::Plain),
            group: Some("Authentication"),
        },
    ]
}

fn active_directory_ldaps_field_definitions() -> Vec<FieldDefinition> {
    vec![
        FieldDefinition {
            id: "bind_dn",
            label: "Bind DN",
            field_type: FieldType::String,
            placeholder: Some("CN=Scanopy Reader,OU=Service Accounts,DC=example,DC=com"),
            secret: false,
            optional: false,
            help_text: Some(
                "Distinguished name of a least-privilege, read-only directory account.",
            ),
            options: None,
            default_value: None,
            inline_format: None,
            group: Some("Authentication"),
        },
        FieldDefinition {
            id: "password",
            label: "Password",
            field_type: FieldType::SecretPathOrInline,
            placeholder: None,
            secret: true,
            optional: false,
            help_text: Some("Password for the read-only bind account."),
            options: None,
            default_value: None,
            inline_format: Some(InlineFormat::Plain),
            group: Some("Authentication"),
        },
        FieldDefinition {
            id: "port",
            label: "LDAPS Port",
            field_type: FieldType::String,
            placeholder: Some("636"),
            secret: false,
            optional: false,
            help_text: Some("TLS-protected LDAP port. Plain LDAP and StartTLS are not used."),
            options: None,
            default_value: Some("636"),
            inline_format: None,
            group: Some("Connection"),
        },
        FieldDefinition {
            id: "server_name",
            label: "TLS Server Name",
            field_type: FieldType::String,
            placeholder: Some("dc01.example.com"),
            secret: false,
            optional: false,
            help_text: Some(
                "DNS name verified against the controller certificate; traffic still goes only to the assigned host IP.",
            ),
            options: None,
            default_value: None,
            inline_format: None,
            group: Some("TLS"),
        },
        FieldDefinition {
            id: "ca_certificate",
            label: "CA Certificate",
            field_type: FieldType::PathOrInline,
            placeholder: Some("-----BEGIN CERTIFICATE-----"),
            secret: false,
            optional: true,
            help_text: Some(
                "Optional private CA certificate, inline or from a daemon-local file. Certificate verification is always enabled.",
            ),
            options: None,
            default_value: None,
            inline_format: Some(InlineFormat::PemCertificate),
            group: Some("TLS"),
        },
        FieldDefinition {
            id: "base_dn",
            label: "Base DN",
            field_type: FieldType::String,
            placeholder: Some("DC=example,DC=com"),
            secret: false,
            optional: false,
            help_text: Some("Directory naming context used for bounded inventory searches."),
            options: None,
            default_value: None,
            inline_format: None,
            group: Some("Collection"),
        },
        FieldDefinition {
            id: "group_dns",
            label: "Group DNs",
            field_type: FieldType::Text,
            placeholder: Some("CN=IT Admins,OU=Groups,DC=example,DC=com"),
            secret: false,
            optional: true,
            help_text: Some(
                "Optional group DNs, one per line (maximum 16). Membership is never collected for unlisted groups.",
            ),
            options: None,
            default_value: None,
            inline_format: None,
            group: Some("Collection"),
        },
    ]
}

fn active_directory_kerberos_field_definitions() -> Vec<FieldDefinition> {
    let mut fields = vec![
        FieldDefinition {
            id: "principal",
            label: "Kerberos Principal",
            field_type: FieldType::String,
            placeholder: Some("scanopy-reader@EXAMPLE.COM"),
            secret: false,
            optional: false,
            help_text: Some(
                "Exact initiating principal that must already exist in the daemon's system credential cache.",
            ),
            options: None,
            default_value: None,
            inline_format: None,
            group: Some("Authentication"),
        },
        FieldDefinition {
            id: "use_system_ccache",
            label: "Use daemon system credential cache",
            field_type: FieldType::Boolean,
            placeholder: None,
            secret: false,
            optional: false,
            help_text: Some(
                "Required acknowledgement: the daemon reads one administrator-managed cache mounted read-only. Scanopy never creates, renews, copies, or deletes tickets.",
            ),
            options: None,
            default_value: Some("false"),
            inline_format: None,
            group: Some("Authentication"),
        },
    ];
    // Kerberos and password transports intentionally share exactly the same
    // endpoint, TLS, and bounded collection controls.
    fields.extend(
        active_directory_ldaps_field_definitions()
            .into_iter()
            .skip(2),
    );
    fields
}

fn ssh_connection_field_definitions() -> Vec<FieldDefinition> {
    vec![
        FieldDefinition {
            id: "username",
            label: "Username",
            field_type: FieldType::String,
            placeholder: Some("scanopy"),
            secret: false,
            optional: false,
            help_text: Some("Account restricted to the documented read-only command set."),
            options: None,
            default_value: None,
            inline_format: None,
            group: Some("Authentication"),
        },
        FieldDefinition {
            id: "port",
            label: "SSH Port",
            field_type: FieldType::String,
            placeholder: Some("22"),
            secret: false,
            optional: false,
            help_text: Some("TCP port exposed by the SSH service."),
            options: None,
            default_value: Some("22"),
            inline_format: None,
            group: Some("Connection"),
        },
        FieldDefinition {
            id: "platform",
            label: "Platform",
            field_type: FieldType::Select,
            placeholder: None,
            secret: false,
            optional: false,
            help_text: Some("Selects the fixed read-only command allowlist; it is never guessed."),
            options: Some(super::ssh::SshPlatform::OPTIONS),
            default_value: Some("Linux"),
            inline_format: None,
            group: Some("Collection"),
        },
        FieldDefinition {
            id: "host_key_policy",
            label: "Host Key Policy",
            field_type: FieldType::Select,
            placeholder: None,
            secret: false,
            optional: false,
            help_text: Some(
                "Strict requires a daemon-local known_hosts file. Accept unknown only for explicitly trusted bootstrap environments.",
            ),
            options: Some(super::ssh::SshHostKeyPolicy::OPTIONS),
            default_value: Some("Strict"),
            inline_format: None,
            group: Some("Host Verification"),
        },
        FieldDefinition {
            id: "known_hosts_file",
            label: "Known Hosts File",
            field_type: FieldType::String,
            placeholder: Some("/root/.ssh/known_hosts"),
            secret: false,
            optional: true,
            help_text: Some(
                "Absolute path on the daemon host; required by strict host-key verification.",
            ),
            options: None,
            default_value: None,
            inline_format: None,
            group: Some("Host Verification"),
        },
    ]
}

/// Optional, non-secret socket-path field for local container-socket credentials. The
/// `placeholder` shows the runtime's default socket so it differs for Docker vs Podman. Blank ⇒
/// the daemon auto-detects, so a socket credential needs no required config.
fn socket_path_field(placeholder: &'static str, help: &'static str) -> FieldDefinition {
    FieldDefinition {
        id: "socket_path",
        label: "Socket Path",
        field_type: FieldType::String,
        placeholder: Some(placeholder),
        secret: false,
        optional: true,
        help_text: Some(help),
        options: None,
        default_value: None,
        inline_format: None,
        group: Some("Connection"),
    }
}

/// Shared field definitions for a container-runtime (Docker/Podman) proxy
/// credential. Only the port label/help differ between runtimes; the path and
/// TLS fields are identical because both speak the Docker-compatible API.
fn container_proxy_field_definitions(
    port_label: &'static str,
    port_help: &'static str,
) -> Vec<FieldDefinition> {
    vec![
        FieldDefinition {
            id: "port",
            label: port_label,
            field_type: FieldType::String,
            placeholder: Some("2375"),
            secret: false,
            optional: true,
            help_text: Some(port_help),
            options: None,
            default_value: Some("2375"),
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
            help_text: Some(
                "PEM-encoded client certificate. All three TLS fields (cert, key, CA chain) must be provided together.",
            ),
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
            help_text: Some("PEM private key. All three TLS fields must be provided together."),
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
            help_text: Some(
                "PEM-encoded CA certificate chain. All three TLS fields must be provided together.",
            ),
            options: None,
            default_value: None,
            inline_format: Some(InlineFormat::PemCertificate),
            group: Some("TLS"),
        },
    ]
}

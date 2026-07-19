use serde::{Deserialize, Serialize};

use crate::server::credentials::r#impl::mapping::ResolvableSecret;

pub const fn default_winrm_port() -> u16 {
    5985
}

/// Daemon-bound WinRM credential for a machine-local administrator account
/// (no domain qualifier). Authenticates over NTLMv2; see
/// `daemon::discovery::integration::winrm` for the transport/encryption
/// constraints this implies.
#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct WindowsLocalAccountQueryCredential {
    pub username: String,
    pub password: ResolvableSecret,
    #[serde(default = "default_winrm_port")]
    pub port: u16,
    #[serde(default)]
    pub use_tls: bool,
    #[serde(default)]
    pub accept_invalid_certs: bool,
}

impl std::fmt::Debug for WindowsLocalAccountQueryCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowsLocalAccountQueryCredential")
            .field("username", &self.username)
            .field("password", &"[REDACTED]")
            .field("port", &self.port)
            .field("use_tls", &self.use_tls)
            .field("accept_invalid_certs", &self.accept_invalid_certs)
            .finish()
    }
}

/// Daemon-bound WinRM credential for a domain account, authenticated with
/// NTLM using an explicit domain qualifier rather than a Kerberos ticket —
/// no krb5 configuration or ticket acquisition is required on the daemon.
#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct WindowsDomainAccountQueryCredential {
    pub domain: String,
    pub username: String,
    pub password: ResolvableSecret,
    #[serde(default = "default_winrm_port")]
    pub port: u16,
    #[serde(default)]
    pub use_tls: bool,
    #[serde(default)]
    pub accept_invalid_certs: bool,
}

impl std::fmt::Debug for WindowsDomainAccountQueryCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowsDomainAccountQueryCredential")
            .field("domain", &self.domain)
            .field("username", &self.username)
            .field("password", &"[REDACTED]")
            .field("port", &self.port)
            .field("use_tls", &self.use_tls)
            .field("accept_invalid_certs", &self.accept_invalid_certs)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_account_debug_redacts_password() {
        let credential = WindowsLocalAccountQueryCredential {
            username: "Administrator".to_string(),
            password: ResolvableSecret::Value {
                value: "do-not-log-this".to_string(),
            },
            port: default_winrm_port(),
            use_tls: false,
            accept_invalid_certs: false,
        };
        let debug = format!("{credential:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("do-not-log-this"));
    }

    #[test]
    fn domain_account_debug_redacts_password() {
        let credential = WindowsDomainAccountQueryCredential {
            domain: "EXAMPLE".to_string(),
            username: "svc-scanopy".to_string(),
            password: ResolvableSecret::Value {
                value: "do-not-log-this".to_string(),
            },
            port: default_winrm_port(),
            use_tls: true,
            accept_invalid_certs: false,
        };
        let debug = format!("{credential:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("do-not-log-this"));
    }
}

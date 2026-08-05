use serde::{Deserialize, Serialize};

use crate::server::credentials::r#impl::mapping::{ResolvableSecret, ResolvableValue};

/// Daemon-bound LDAPS credential. Passwords are kept in `ResolvableSecret`, whose
/// `Debug` implementation never exposes the value, and may be sourced from a
/// daemon-local file.
#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct ActiveDirectoryLdapsQueryCredential {
    pub bind_dn: String,
    pub password: ResolvableSecret,
    #[serde(default = "default_ldaps_port")]
    pub port: u16,
    /// DNS name checked against the controller certificate while the TCP
    /// connection is still made to the explicitly targeted host IP.
    pub server_name: String,
    pub base_dn: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<ResolvableValue>,
    /// Newline-separated group DNs. Only these groups have membership queried.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_dns: Option<String>,
}

/// Daemon-bound LDAPS credential using an administrator-managed system Kerberos
/// credential cache. This wire type deliberately has no password, keytab,
/// ticket, cache-content, or ticket-export field.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct ActiveDirectoryKerberosQueryCredential {
    pub principal: String,
    pub use_system_ccache: bool,
    #[serde(default = "default_ldaps_port")]
    pub port: u16,
    /// DNS name used for both TLS certificate validation and the
    /// `ldap/<server_name>` Kerberos service principal. TCP still connects only
    /// to the host IP assigned by the server.
    pub server_name: String,
    pub base_dn: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<ResolvableValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_dns: Option<String>,
}

impl std::fmt::Debug for ActiveDirectoryLdapsQueryCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActiveDirectoryLdapsQueryCredential")
            .field("bind_dn", &self.bind_dn)
            .field("password", &"********")
            .field("port", &self.port)
            .field("server_name", &self.server_name)
            .field("base_dn", &self.base_dn)
            .field("ca_certificate", &self.ca_certificate)
            .field("group_dns", &self.group_dns)
            .finish()
    }
}

pub const fn default_ldaps_port() -> u16 {
    636
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_credential_debug_redacts_password() {
        let credential = ActiveDirectoryLdapsQueryCredential {
            bind_dn: "CN=Scanopy,DC=example,DC=com".to_string(),
            password: ResolvableSecret::Value {
                value: "do-not-log-this".to_string(),
            },
            port: 636,
            server_name: "dc01.example.com".to_string(),
            base_dn: "DC=example,DC=com".to_string(),
            ca_certificate: None,
            group_dns: None,
        };
        let debug = format!("{credential:?}");
        assert!(debug.contains("********"));
        assert!(!debug.contains("do-not-log-this"));
    }

    #[test]
    fn kerberos_wire_contains_no_secret_or_ticket_material() {
        let credential = ActiveDirectoryKerberosQueryCredential {
            principal: "scanopy-reader@EXAMPLE.COM".to_string(),
            use_system_ccache: true,
            port: 636,
            server_name: "dc01.example.com".to_string(),
            base_dn: "DC=example,DC=com".to_string(),
            ca_certificate: None,
            group_dns: None,
        };
        let json = serde_json::to_value(credential).unwrap();
        assert_eq!(json["principal"], "scanopy-reader@EXAMPLE.COM");
        assert_eq!(json["use_system_ccache"], true);
        for forbidden in ["password", "keytab", "ticket", "ccache"] {
            assert!(json.get(forbidden).is_none());
        }
    }
}

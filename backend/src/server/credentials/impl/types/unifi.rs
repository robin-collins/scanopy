use serde::{Deserialize, Serialize};
use strum_macros::VariantNames;
use utoipa::ToSchema;

use super::super::mapping::ResolvableSecret;
use super::SelectOption;

#[derive(
    Debug, Clone, Copy, Default, Serialize, Deserialize, Eq, PartialEq, Hash, ToSchema, VariantNames,
)]
pub enum UnifiApiType {
    #[default]
    Modern,
    Legacy,
}

impl UnifiApiType {
    pub const OPTIONS: &'static [SelectOption] = &[
        SelectOption {
            value: "Modern",
            label: "UniFi OS (Modern)",
        },
        SelectOption {
            value: "Legacy",
            label: "Legacy Controller",
        },
    ];
}

#[derive(
    Debug, Clone, Copy, Default, Serialize, Deserialize, Eq, PartialEq, Hash, ToSchema, VariantNames,
)]
pub enum UnifiTlsPolicy {
    #[default]
    Verify,
    AllowInvalidCertificate,
}

impl UnifiTlsPolicy {
    pub const OPTIONS: &'static [SelectOption] = &[
        SelectOption {
            value: "Verify",
            label: "Verify Certificate",
        },
        SelectOption {
            value: "AllowInvalidCertificate",
            label: "Allow Invalid Certificate",
        },
    ];
}

/// Daemon-bound UniFi credential. The request URL retains the configured DNS
/// identity for TLS, while the daemon transport resolves it only to the host IP
/// selected by the discovery pipeline.
#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct UnifiQueryCredential {
    pub controller_url: String,
    pub server_name: String,
    pub site: String,
    pub api_type: UnifiApiType,
    pub tls_policy: UnifiTlsPolicy,
    pub username: String,
    pub password: ResolvableSecret,
}

impl std::fmt::Debug for UnifiQueryCredential {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("UnifiQueryCredential")
            .field("controller_url", &self.controller_url)
            .field("server_name", &self.server_name)
            .field("site", &self.site)
            .field("api_type", &self.api_type)
            .field("tls_policy", &self.tls_policy)
            .field("username", &self.username)
            .field("password", &"********")
            .finish()
    }
}

impl UnifiQueryCredential {
    pub fn port(&self) -> Option<u16> {
        url::Url::parse(&self.controller_url)
            .ok()
            .and_then(|url| url.port_or_known_default())
    }
}

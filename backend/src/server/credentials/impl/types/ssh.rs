use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, IntoStaticStr, VariantNames};
use utoipa::ToSchema;

use crate::server::credentials::r#impl::mapping::ResolvableSecret;

use super::SelectOption;

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    ToSchema,
    Display,
    EnumIter,
    IntoStaticStr,
    VariantNames,
)]
pub enum SshPlatform {
    #[default]
    Linux,
    CiscoIos,
    HpComware,
    ArubaAos,
}

impl SshPlatform {
    pub const OPTIONS: &'static [SelectOption] = &[
        SelectOption {
            value: "Linux",
            label: "Linux / Unix",
        },
        SelectOption {
            value: "CiscoIos",
            label: "Cisco IOS",
        },
        SelectOption {
            value: "HpComware",
            label: "HP/HPE Comware",
        },
        SelectOption {
            value: "ArubaAos",
            label: "ArubaOS-Switch",
        },
    ];
}

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    ToSchema,
    Display,
    EnumIter,
    IntoStaticStr,
    VariantNames,
)]
pub enum SshHostKeyPolicy {
    #[default]
    Strict,
    AcceptUnknown,
}

impl SshHostKeyPolicy {
    pub const OPTIONS: &'static [SelectOption] = &[
        SelectOption {
            value: "Strict",
            label: "Strict (known_hosts)",
        },
        SelectOption {
            value: "AcceptUnknown",
            label: "Accept unknown key",
        },
    ];
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "method")]
pub enum SshAuthentication {
    Password {
        password: ResolvableSecret,
    },
    PrivateKey {
        private_key: ResolvableSecret,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        passphrase: Option<ResolvableSecret>,
    },
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SshQueryCredential {
    pub username: String,
    pub authentication: SshAuthentication,
    pub port: u16,
    pub platform: SshPlatform,
    pub host_key_policy: SshHostKeyPolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub known_hosts_file: Option<String>,
}

impl std::fmt::Debug for SshQueryCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SshQueryCredential")
            .field("username", &self.username)
            .field("authentication", &"[REDACTED]")
            .field("port", &self.port)
            .field("platform", &self.platform)
            .field("host_key_policy", &self.host_key_policy)
            .field("known_hosts_file", &self.known_hosts_file)
            .finish()
    }
}

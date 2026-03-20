use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use strum::IntoDiscriminant;
use strum::IntoStaticStr;
use strum_macros::EnumIter;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::server::shared::{
    concepts::Concept,
    types::{
        Color, Icon,
        metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
    },
};

use super::{CredentialType, SecretValue, default_docker_port};

/// Category grouping for credential types.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, IntoStaticStr, ToSchema, PartialEq, Eq)]
pub enum CredentialCategory {
    /// Network monitoring protocols (SNMP, NetFlow, sFlow)
    #[strum(serialize = "Network Monitoring")]
    NetworkMonitoring,
    /// Container and virtualization platforms (Docker, vSphere, ESXi)
    #[strum(serialize = "Container & Virtualization")]
    ContainerVirtualization,
}

/// How a credential is scoped to targets.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub enum ScopeModel {
    /// Network default — try on all hosts with matching open ports
    Broadcast,
    /// Assigned to specific hosts only
    PerHost,
}

/// Provide an iterator over all `CredentialType` variants (with default field values).
/// Used by `generate-fixtures` to produce `credential-types.json`.
#[derive(EnumIter)]
pub enum CredentialTypeVariant {
    SnmpV2c,
    DockerProxy,
}

impl CredentialTypeVariant {
    pub fn to_credential_type(&self) -> CredentialType {
        match self {
            Self::SnmpV2c => CredentialType::SnmpV2c {
                community: SecretValue::Inline {
                    value: SecretString::from(String::new()),
                },
            },
            Self::DockerProxy => CredentialType::DockerProxy {
                port: default_docker_port(),
                path: None,
                ssl_cert: None,
                ssl_key: None,
                ssl_chain: None,
            },
        }
    }
}

/// A credential assigned to a host, optionally limited to specific interfaces.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct CredentialAssignment {
    pub credential_id: Uuid,
    /// Interface IDs to limit this credential to. None = all host interfaces.
    #[serde(default)]
    #[schema(required)]
    pub interface_ids: Option<Vec<Uuid>>,
}

impl HasId for CredentialType {
    fn id(&self) -> &'static str {
        self.discriminant().into()
    }
}

impl EntityMetadataProvider for CredentialType {
    fn color(&self) -> Color {
        match self {
            Self::SnmpV2c { .. } => Concept::SNMP.color(),
            Self::DockerProxy { .. } => Concept::Virtualization.color(),
        }
    }
    fn icon(&self) -> Icon {
        match self {
            Self::SnmpV2c { .. } => Concept::SNMP.icon(),
            Self::DockerProxy { .. } => Concept::Virtualization.icon(),
        }
    }
}

impl TypeMetadataProvider for CredentialType {
    fn name(&self) -> &'static str {
        match self {
            Self::SnmpV2c { .. } => "SNMP v2c",
            Self::DockerProxy { .. } => "Docker Proxy",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::SnmpV2c { .. } => "SNMPv2c community string for querying network devices",
            Self::DockerProxy { .. } => "Docker API proxy credentials. TLS is optional.",
        }
    }

    fn category(&self) -> &'static str {
        self.credential_category().into()
    }

    fn metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "fields": self.field_definitions(),
            "port_description": self.port_description(),
            "custom_port_field": self.custom_port_field(),
            "scope_models": self.scope_models(),
        })
    }
}

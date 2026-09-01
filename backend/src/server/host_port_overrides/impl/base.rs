use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::shared::entities::ChangeTriggersTopologyStaleness;

/// Discriminator for the tagged-union catalogue reference. A bare string would
/// let a future upstream release add a built-in port/service with the same name
/// as an existing custom entry, silently changing what every stored override
/// points at. The kind plus the id pins which namespace the reference lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash, ToSchema)]
pub enum ServiceRefKind {
    BuiltIn,
    Custom,
}

/// The base data for a HostPortOverride entity (everything except id/created_at/updated_at).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Validate, ToSchema)]
pub struct HostPortOverrideBase {
    /// The host this override belongs to.
    pub host_id: Uuid,
    /// Denormalized from the host's network_id — see the migration comment;
    /// required for the generic network-scoped access-control filter.
    pub network_id: Uuid,
    /// Port number this override applies to. Port 0 is never a host port, so
    /// the range matches the Known Ports catalogue (1..=65535).
    #[validate(range(min = 1, max = 65535))]
    #[schema(minimum = 1, maximum = 65535)]
    pub port_number: i64,
    /// Transport protocol this override applies to.
    pub port_protocol: String,
    /// Per-host display/service name. NULL = fall back to the global default.
    pub display_name: Option<String>,
    /// Per-host icon URL. NULL = use the default icon.
    pub icon_url: Option<String>,
    /// Catalogue-reference discriminator. NULL together with `service_ref_id` =
    /// not assigned. Carries no FK; validated at the API.
    pub service_ref_kind: Option<ServiceRefKind>,
    /// Catalogue id — a built-in ServiceDefinition id string OR a custom row
    /// UUID, depending on `service_ref_kind`. No FK; validated at the API.
    pub service_ref_id: Option<String>,
}

impl Default for HostPortOverrideBase {
    fn default() -> Self {
        Self {
            host_id: Uuid::nil(),
            network_id: Uuid::nil(),
            port_number: 0,
            port_protocol: "Tcp".to_string(),
            display_name: None,
            icon_url: None,
            service_ref_kind: None,
            service_ref_id: None,
        }
    }
}

/// A per-host display override for a single port (well-known or unclaimed).
/// Keyed on the value tuple (host_id, port_number, port_protocol) rather than
/// the port row UUID so overrides survive rescans that recreate port rows.
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
pub struct HostPortOverride {
    /// Server-assigned unique identifier.
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    /// When this override was first created.
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    /// When this override was last modified.
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: HostPortOverrideBase,
}

impl Display for HostPortOverride {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "host {} {} {}/{}",
            self.base.host_id, self.base.port_number, self.base.port_protocol, self.id
        )
    }
}

impl ChangeTriggersTopologyStaleness<HostPortOverride> for HostPortOverride {
    fn triggers_staleness(&self, _other: Option<HostPortOverride>) -> bool {
        false
    }
}

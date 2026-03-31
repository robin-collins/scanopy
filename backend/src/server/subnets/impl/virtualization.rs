use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Virtualization metadata for subnets that belong to a virtual infrastructure.
/// Consistent with HostVirtualization and ServiceVirtualization patterns.
/// Points to the service that provides the virtualization (e.g., Docker daemon).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(tag = "type")]
pub enum SubnetVirtualization {
    /// Docker bridge network — host-scoped, same CIDR on different hosts are distinct subnets.
    #[schema(title = "Docker")]
    Docker(DockerSubnetVirtualization),
}

impl SubnetVirtualization {
    pub fn service_id(&self) -> Option<Uuid> {
        match self {
            Self::Docker(d) => Some(d.service_id),
        }
    }

    pub fn set_service_id(&mut self, id: Uuid) {
        match self {
            Self::Docker(d) => d.service_id = id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct DockerSubnetVirtualization {
    /// The Docker daemon service that owns this bridge network.
    /// Different Docker daemons on different hosts = distinct bridge subnets.
    pub service_id: Uuid,
}

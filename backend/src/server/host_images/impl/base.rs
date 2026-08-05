use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::shared::entities::ChangeTriggersTopologyStaleness;

/// The base data for a HostImage entity (everything except id/created_at/updated_at).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Validate, ToSchema)]
pub struct HostImageBase {
    pub host_id: Uuid,
    /// Denormalized from the host's network_id — see the migration comment;
    /// required for the generic network-scoped access-control filter.
    pub network_id: Uuid,
    pub filename: String,
    pub content_type: String,
    /// i64 rather than u64 — sqlx/Postgres BIGINT is signed; sizes here are
    /// bounded well under i64::MAX by the upload handler's size limit anyway.
    pub size_bytes: i64,
    /// Path relative to the server's configured data directory, not an
    /// absolute filesystem path — see `host_images::service` for resolution.
    #[schema(read_only)]
    pub storage_path: String,
}

impl Default for HostImageBase {
    fn default() -> Self {
        Self {
            host_id: Uuid::nil(),
            network_id: Uuid::nil(),
            filename: String::new(),
            content_type: String::new(),
            size_bytes: 0,
            storage_path: String::new(),
        }
    }
}

/// A single uploaded image for a host, part of that host's image gallery.
/// One gallery image may additionally be selected via
/// `Host.topology_icon_image_id` as the host's topology node icon.
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
pub struct HostImage {
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: HostImageBase,
}

impl Display for HostImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (ID: {})", self.base.filename, self.id)
    }
}

impl ChangeTriggersTopologyStaleness<HostImage> for HostImage {
    fn triggers_staleness(&self, _other: Option<HostImage>) -> bool {
        false
    }
}

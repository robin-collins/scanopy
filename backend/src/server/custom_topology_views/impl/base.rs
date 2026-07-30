use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::shared::entities::ChangeTriggersTopologyStaleness;

/// The base data for a CustomTopologyView entity (everything except id/created_at/updated_at).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Validate, ToSchema)]
pub struct CustomTopologyViewBase {
    pub network_id: Uuid,
    #[validate(length(
        min = 1,
        max = 100,
        message = "Name must be between 1 and 100 characters"
    ))]
    pub name: String,
}

impl Default for CustomTopologyViewBase {
    fn default() -> Self {
        Self {
            network_id: Uuid::nil(),
            name: "New View".to_string(),
        }
    }
}

/// A user-authored topology view: unlike the built-in L2/L3/Workloads/
/// Application views (computed live from entity data), a custom view's nodes
/// and edges (`CustomViewNode`/`CustomViewEdge`) are hand-placed by the user
/// and persisted as-is.
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
pub struct CustomTopologyView {
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
    pub base: CustomTopologyViewBase,
}

impl Display for CustomTopologyView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (ID: {})", self.base.name, self.id)
    }
}

impl ChangeTriggersTopologyStaleness<CustomTopologyView> for CustomTopologyView {
    fn triggers_staleness(&self, _other: Option<CustomTopologyView>) -> bool {
        false
    }
}

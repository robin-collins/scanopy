use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::ports::r#impl::base::TransportProtocol;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum CatalogueSource {
    BuiltIn,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct KnownPort {
    /// PortType discriminant for a built-in, UUID text for a custom entry.
    pub id: String,
    #[schema(read_only)]
    pub organization_id: Option<Uuid>,
    #[schema(read_only)]
    pub source: CatalogueSource,
    pub name: String,
    pub description: Option<String>,
    #[schema(minimum = 1, maximum = 65535)]
    pub port_number: u16,
    pub transport_protocol: TransportProtocol,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, Validate)]
pub struct KnownPortInput {
    #[validate(length(
        min = 1,
        max = 100,
        message = "Name must be between 1 and 100 characters"
    ))]
    pub name: String,
    #[validate(length(max = 500, message = "Description must be 500 characters or fewer"))]
    pub description: Option<String>,
    #[validate(range(min = 1, max = 65535))]
    #[schema(minimum = 1, maximum = 65535)]
    pub port_number: u16,
    pub transport_protocol: TransportProtocol,
}

impl KnownPortInput {
    pub fn normalized(mut self) -> Self {
        self.name = self.name.trim().to_string();
        self.description = self
            .description
            .map(|description| description.trim().to_string())
            .filter(|description| !description.is_empty());
        self
    }
}

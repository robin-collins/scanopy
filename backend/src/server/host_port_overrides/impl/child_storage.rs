use uuid::Uuid;

use crate::server::{
    host_port_overrides::r#impl::base::HostPortOverride,
    shared::storage::child::ChildStorableEntity,
};

impl ChildStorableEntity for HostPortOverride {
    fn parent_column() -> &'static str {
        "host_id"
    }

    fn parent_id(&self) -> Uuid {
        self.base.host_id
    }
}

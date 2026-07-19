use uuid::Uuid;

use crate::server::{
    host_images::r#impl::base::HostImage, shared::storage::child::ChildStorableEntity,
};

impl ChildStorableEntity for HostImage {
    fn parent_column() -> &'static str {
        "host_id"
    }

    fn parent_id(&self) -> Uuid {
        self.base.host_id
    }
}

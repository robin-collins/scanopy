use uuid::Uuid;

use crate::server::{
    custom_view_nodes::r#impl::base::CustomViewNode, shared::storage::child::ChildStorableEntity,
};

impl ChildStorableEntity for CustomViewNode {
    fn parent_column() -> &'static str {
        "view_id"
    }

    fn parent_id(&self) -> Uuid {
        self.base.view_id
    }
}

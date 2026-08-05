use uuid::Uuid;

use crate::server::{
    custom_view_edges::r#impl::base::CustomViewEdge, shared::storage::child::ChildStorableEntity,
};

impl ChildStorableEntity for CustomViewEdge {
    fn parent_column() -> &'static str {
        "view_id"
    }

    fn parent_id(&self) -> Uuid {
        self.base.view_id
    }
}

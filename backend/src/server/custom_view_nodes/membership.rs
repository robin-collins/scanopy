use std::collections::{HashMap, HashSet};

use uuid::Uuid;
use validator::Validate;

use crate::server::{
    custom_view_nodes::r#impl::{base::CustomViewNode, types::NodeKind},
    shared::types::api::ValidationError,
};

pub fn validate_node_fields(node: &CustomViewNode) -> Result<(), ValidationError> {
    node.validate()
        .map_err(|error| ValidationError::new(format!("Invalid custom view node: {error}")))
}

/// Validate the complete membership graph for one custom topology view.
///
/// Callers must pass the final graph (after applying all proposed changes),
/// not a sequence of partial updates. That makes group-kind changes and
/// multi-node reparenting validate atomically.
pub fn validate_membership_graph(
    nodes: &[CustomViewNode],
    view_id: Uuid,
) -> Result<(), ValidationError> {
    let mut by_id = HashMap::with_capacity(nodes.len());
    for node in nodes {
        if node.base.view_id != view_id {
            return Err(ValidationError::new(
                "Every node in a layout change must belong to the target custom view",
            ));
        }
        if by_id.insert(node.id, node).is_some() {
            return Err(ValidationError::new(
                "A layout change cannot contain duplicate node IDs",
            ));
        }
    }

    // Detect cycles independently of kind rules so malformed graphs receive
    // an explicit cycle rejection rather than relying on today's Group-only
    // parent rule to make cycles structurally impossible.
    for node in nodes {
        let mut visited = HashSet::new();
        let mut current = node;
        while let Some(parent_id) = current.base.parent_node_id {
            if !visited.insert(current.id) || parent_id == node.id {
                return Err(ValidationError::new(
                    "Custom view node parent relationships cannot form a cycle",
                ));
            }
            let Some(parent) = by_id.get(&parent_id) else {
                break;
            };
            current = parent;
        }
    }

    for node in nodes {
        let Some(parent_id) = node.base.parent_node_id else {
            continue;
        };

        if node.base.kind == NodeKind::Group {
            return Err(ValidationError::new(
                "Group frames cannot be children of other nodes",
            ));
        }

        let parent = by_id.get(&parent_id).ok_or_else(|| {
            ValidationError::new(
                "Parent nodes must exist and belong to the same custom view as their child",
            )
        })?;
        if parent.base.kind != NodeKind::Group {
            return Err(ValidationError::new(
                "Only group frames may be used as parent nodes",
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::custom_view_nodes::r#impl::base::CustomViewNodeBase;

    fn node(
        id: Uuid,
        view_id: Uuid,
        kind: NodeKind,
        parent_node_id: Option<Uuid>,
    ) -> CustomViewNode {
        CustomViewNode {
            id,
            base: CustomViewNodeBase {
                view_id,
                kind,
                parent_node_id,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn accepts_non_group_children_of_a_group() {
        let view_id = Uuid::new_v4();
        let group_id = Uuid::new_v4();
        let nodes = vec![
            node(group_id, view_id, NodeKind::Group, None),
            node(Uuid::new_v4(), view_id, NodeKind::Text, Some(group_id)),
        ];

        assert!(validate_membership_graph(&nodes, view_id).is_ok());
    }

    #[test]
    fn rejects_group_nesting() {
        let view_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();
        let nodes = vec![
            node(parent_id, view_id, NodeKind::Group, None),
            node(Uuid::new_v4(), view_id, NodeKind::Group, Some(parent_id)),
        ];

        let error = validate_membership_graph(&nodes, view_id).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Group frames cannot be children")
        );
    }

    #[test]
    fn rejects_cycles_even_when_parent_kinds_are_invalid() {
        let view_id = Uuid::new_v4();
        let first_id = Uuid::new_v4();
        let second_id = Uuid::new_v4();
        let nodes = vec![
            node(first_id, view_id, NodeKind::Text, Some(second_id)),
            node(second_id, view_id, NodeKind::Text, Some(first_id)),
        ];

        let error = validate_membership_graph(&nodes, view_id).unwrap_err();
        assert!(error.to_string().contains("cannot form a cycle"));
    }

    #[test]
    fn rejects_self_parenting() {
        let view_id = Uuid::new_v4();
        let node_id = Uuid::new_v4();
        let nodes = vec![node(node_id, view_id, NodeKind::Text, Some(node_id))];

        let error = validate_membership_graph(&nodes, view_id).unwrap_err();
        assert!(error.to_string().contains("cannot form a cycle"));
    }

    #[test]
    fn rejects_missing_or_cross_view_parents() {
        let view_id = Uuid::new_v4();
        let nodes = vec![node(
            Uuid::new_v4(),
            view_id,
            NodeKind::Entity,
            Some(Uuid::new_v4()),
        )];

        let error = validate_membership_graph(&nodes, view_id).unwrap_err();
        assert!(error.to_string().contains("exist and belong to the same"));
    }

    #[test]
    fn rejects_non_group_parents() {
        let view_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();
        let nodes = vec![
            node(parent_id, view_id, NodeKind::Text, None),
            node(Uuid::new_v4(), view_id, NodeKind::Entity, Some(parent_id)),
        ];

        let error = validate_membership_graph(&nodes, view_id).unwrap_err();
        assert!(error.to_string().contains("Only group frames"));
    }
}

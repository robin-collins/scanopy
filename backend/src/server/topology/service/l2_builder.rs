use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use super::{
    context::TopologyContext,
    element_rules::{
        ElementMatchData, TaggableLookups, apply_element_rules, resolve_element_tag_ids,
    },
    view::ViewBuilder,
};
use crate::server::{
    interfaces::r#impl::base::Neighbor,
    shared::entities::EntityDiscriminants,
    topology::types::{
        edges::{DiscoveryProtocol, Edge, EdgeHandle, EdgeType, EdgeViewConfig},
        grouping::GroupingConfig,
        nodes::{ContainerType, ElementEntityType, Node, NodeType},
    },
};

/// if_type values to exclude from L2 view (virtual/software ip_addresses)
const EXCLUDED_IF_TYPES: &[i32] = &[
    24,  // softwareLoopback
    53,  // propVirtual
    71,  // ieee80211 (Wi-Fi)
    131, // tunnel
    135, // l2vlan
    136, // l3ipvlan
    209, // bridge
];

pub struct L2Builder;

impl L2Builder {
    /// Generate a deterministic container UUID from host_id for the L2 view.
    fn container_id_for_host(host_id: Uuid) -> Uuid {
        Uuid::new_v5(&Uuid::NAMESPACE_OID, format!("l2:{host_id}").as_bytes())
    }
}

impl ViewBuilder for L2Builder {
    fn build(&self, ctx: &TopologyContext, grouping: &GroupingConfig) -> (Vec<Node>, Vec<Edge>) {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        let host_lookup: HashMap<Uuid, &crate::server::hosts::r#impl::base::Host> =
            ctx.hosts.iter().map(|h| (h.id, h)).collect();

        // 1. Build PhysicalLink edges using interface_id as source/target
        //    (unlike create_physical_link_edges which uses ip_address_id)
        let mut processed_pairs: HashSet<(Uuid, Uuid)> = HashSet::new();
        // Hosts targeted by a partial (device-only) neighbor resolution — no
        // specific remote port was identified, so these get a single synthetic
        // "Host" element standing in for the unresolved port.
        let mut host_neighbor_target_ids: HashSet<Uuid> = HashSet::new();

        for source_entry in ctx.get_interfaces_with_neighbor() {
            match &source_entry.base.neighbor {
                Some(Neighbor::Interface(target_interface_id)) => {
                    let target_interface_id = *target_interface_id;

                    // Dedup bidirectional pairs
                    let pair_key = if source_entry.id < target_interface_id {
                        (source_entry.id, target_interface_id)
                    } else {
                        (target_interface_id, source_entry.id)
                    };
                    if !processed_pairs.insert(pair_key) {
                        continue;
                    }

                    let target_entry = match ctx.get_interface_by_id(target_interface_id) {
                        Some(e) => e,
                        None => continue,
                    };

                    // Skip self-loops
                    if source_entry.base.host_id == target_entry.base.host_id {
                        continue;
                    }

                    let label = Some(format!(
                        "{} ↔ {}",
                        source_entry.display_name(),
                        target_entry.display_name()
                    ));

                    edges.push(Edge {
                        id: Uuid::new_v4(),
                        source: source_entry.id, // interface_id, not ip_address_id
                        target: target_entry.id, // interface_id, not ip_address_id
                        edge_type: EdgeType::PhysicalLink {
                            source_entity_id: source_entry.id,
                            target_entity_id: target_entry.id,
                            protocol: DiscoveryProtocol::default(),
                        },
                        label,
                        source_handle: EdgeHandle::Bottom,
                        target_handle: EdgeHandle::Top,
                        is_multi_hop: false,
                        view_config: EdgeViewConfig::default(),
                    });
                }
                Some(Neighbor::Host(target_host_id)) => {
                    let target_host_id = *target_host_id;

                    // Skip self-loops
                    if source_entry.base.host_id == target_host_id {
                        continue;
                    }

                    let Some(target_host) = host_lookup.get(&target_host_id) else {
                        continue;
                    };

                    // Dedup bidirectional pairs
                    let pair_key = if source_entry.id < target_host_id {
                        (source_entry.id, target_host_id)
                    } else {
                        (target_host_id, source_entry.id)
                    };
                    if !processed_pairs.insert(pair_key) {
                        continue;
                    }

                    let label = Some(format!(
                        "{} ↔ {}",
                        source_entry.display_name(),
                        target_host.base.name
                    ));

                    edges.push(Edge {
                        id: Uuid::new_v4(),
                        source: source_entry.id, // interface_id
                        target: target_host_id,  // host_id — no specific port resolved
                        edge_type: EdgeType::PhysicalLink {
                            source_entity_id: source_entry.id,
                            target_entity_id: target_host_id,
                            protocol: DiscoveryProtocol::default(),
                        },
                        label,
                        source_handle: EdgeHandle::Bottom,
                        target_handle: EdgeHandle::Top,
                        is_multi_hop: false,
                        view_config: EdgeViewConfig::default(),
                    });

                    host_neighbor_target_ids.insert(target_host_id);
                }
                None => continue,
            }
        }

        // 2. Determine qualifying hosts:
        //    - Hosts with any Interface that has LLDP/CDP neighbor data
        //    - Hosts that are targets of physical links (by interface or, for
        //      partial resolutions, by host directly)
        let mut qualifying_host_ids: HashSet<Uuid> = HashSet::new();

        // Hosts with neighbor data
        for entry in ctx.get_interfaces_with_neighbor() {
            qualifying_host_ids.insert(entry.base.host_id);
        }

        // Hosts that are targets (look up target interface → host_id)
        for edge in &edges {
            if let EdgeType::PhysicalLink {
                target_entity_id, ..
            } = &edge.edge_type
            {
                if let Some(entry) = ctx.get_interface_by_id(*target_entity_id) {
                    qualifying_host_ids.insert(entry.base.host_id);
                } else if host_lookup.contains_key(target_entity_id) {
                    qualifying_host_ids.insert(*target_entity_id);
                }
            }
        }

        // 3. Create Host containers for qualifying hosts
        for &host_id in &qualifying_host_ids {
            let Some(host) = host_lookup.get(&host_id) else {
                continue;
            };

            let container_id = Self::container_id_for_host(host_id);
            nodes.push(Node {
                id: container_id,
                node_type: NodeType::Container {
                    container_type: ContainerType::Host,
                    parent_container_id: None,
                    entity_id: Some(host_id),
                    icon: None,
                    color: None,
                    associated_service_definition: None,
                    element_rule_id: None,
                    will_accept_edges: false,
                },
                position: Default::default(),
                size: Default::default(),
                header: Some(host.base.name.clone()),
            });
        }

        // 4. Create Port elements for qualifying hosts' IfEntries
        for &host_id in &qualifying_host_ids {
            let container_id = Self::container_id_for_host(host_id);
            for entry in ctx.get_interfaces_for_host(host_id) {
                // Skip virtual/software interface types
                if EXCLUDED_IF_TYPES.contains(&entry.base.if_type) {
                    continue;
                }

                let mut node = Node::element(
                    entry.id,
                    container_id,
                    host_id,
                    ElementEntityType::Interface {
                        interface_id: entry.id,
                    },
                );
                node.header = Some(entry.display_name().to_string());
                nodes.push(node);
            }
        }

        // 4b. Synthetic "Host" elements standing in for the unresolved port on
        // hosts we only matched by device identity (chassis MAC/sysName), not
        // by a specific remote interface — e.g. a host with no polled
        // Interface records of its own. The node id equals the host_id,
        // matching the edge target set above and the Host-element convention
        // used elsewhere in the topology builders.
        for &host_id in &host_neighbor_target_ids {
            let Some(host) = host_lookup.get(&host_id) else {
                continue;
            };
            let container_id = Self::container_id_for_host(host_id);
            let mut node =
                Node::element(host_id, container_id, host_id, ElementEntityType::Host {});
            node.header = Some(host.base.name.clone());
            nodes.push(node);
        }

        // 5. Apply element rules (ByTag already has L2Physical in applicable_views)
        let if_entry_lookup: std::collections::HashMap<
            Uuid,
            &crate::server::interfaces::r#impl::base::Interface,
        > = ctx.interfaces.iter().map(|e| (e.id, e)).collect();
        let tag_lookups = TaggableLookups {
            hosts: Some(&host_lookup),
            services: None,
            subnets: None,
        };
        let _ = apply_element_rules(
            &mut nodes,
            &grouping.element_rules,
            |node| {
                if let NodeType::Element { host_id, .. } = &node.node_type {
                    let tag_ids = resolve_element_tag_ids(
                        EntityDiscriminants::Interface,
                        *host_id,
                        &tag_lookups,
                    );
                    let interface = if_entry_lookup.get(&node.id);
                    let native_vlan_id = interface.and_then(|e| e.base.native_vlan_id);
                    let resolved_vlan = native_vlan_id.and_then(|vid| ctx.get_vlan_by_id(vid));
                    Some(ElementMatchData {
                        categories: HashSet::new(),
                        tag_ids,
                        element_entity: EntityDiscriminants::Interface,
                        virtualizer_service_id: None,
                        deployment_group: None,
                        native_vlan_id,
                        vlan_number: resolved_vlan.map(|v| v.base.vlan_number),
                        vlan_name: resolved_vlan.map(|v| v.base.name.clone()),
                        is_trunk_port: interface
                            .and_then(|e| e.base.vlan_ids.as_ref())
                            .is_some_and(|v| !v.is_empty()),
                        oper_status: interface.map(|e| e.base.oper_status),
                    })
                } else {
                    None
                }
            },
            None,
            None,
        );

        (nodes, edges)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::{
        hosts::r#impl::base::{Host, HostBase},
        interfaces::r#impl::base::{Interface, InterfaceBase, Neighbor},
        topology::{
            service::context::TopologyContext,
            types::{
                base::TopologyOptions,
                grouping::GroupingConfig,
                nodes::{ContainerType, NodeType},
            },
        },
    };
    use chrono::Utc;

    fn make_host(name: &str) -> Host {
        Host {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: HostBase {
                name: name.to_string(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn make_if_entry(
        host_id: Uuid,
        if_index: i32,
        if_type: i32,
        neighbor: Option<Neighbor>,
    ) -> Interface {
        Interface {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: InterfaceBase {
                host_id,
                if_index,
                if_descr: format!("GigabitEthernet0/{if_index}"),
                if_name: Some(format!("Gi0/{if_index}")),
                if_type,
                speed_bps: Some(1_000_000_000),
                neighbor,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn l2_grouping() -> GroupingConfig {
        GroupingConfig {
            container_rules: vec![],
            element_rules: vec![],
        }
    }

    #[test]
    fn test_empty_topology() {
        let options = TopologyOptions::default();
        let ctx = TopologyContext::new(
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &options,
            crate::server::topology::types::views::TopologyView::L3Logical,
        );
        let builder = L2Builder;
        let (nodes, edges) = builder.build(&ctx, &l2_grouping());
        assert!(nodes.is_empty());
        assert!(edges.is_empty());
    }

    #[test]
    fn test_hosts_without_neighbors_excluded() {
        let h1 = make_host("server-1");
        let ie1 = make_if_entry(h1.id, 1, 6, None);
        let hosts = vec![h1];
        let interfaces = vec![ie1];
        let options = TopologyOptions::default();
        let ctx = TopologyContext::new(
            &hosts,
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &interfaces,
            &[],
            &[],
            &options,
            crate::server::topology::types::views::TopologyView::L3Logical,
        );

        let builder = L2Builder;
        let (nodes, edges) = builder.build(&ctx, &l2_grouping());
        // No LLDP neighbors → no qualifying hosts → empty
        assert!(nodes.is_empty());
        assert!(edges.is_empty());
    }

    #[test]
    fn test_physical_link_creates_containers_and_edges() {
        let h1 = make_host("switch-1");
        let h2 = make_host("switch-2");

        let ie1 = make_if_entry(h1.id, 1, 6, None);
        let ie2 = make_if_entry(h2.id, 1, 6, None);

        // ie1 has neighbor pointing to ie2
        let mut ie1_with_neighbor = ie1.clone();
        ie1_with_neighbor.base.neighbor = Some(Neighbor::Interface(ie2.id));

        let hosts = vec![h1, h2];
        let interfaces = vec![ie1_with_neighbor, ie2];
        let options = TopologyOptions::default();
        let ctx = TopologyContext::new(
            &hosts,
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &interfaces,
            &[],
            &[],
            &options,
            crate::server::topology::types::views::TopologyView::L3Logical,
        );

        let builder = L2Builder;
        let (nodes, edges) = builder.build(&ctx, &l2_grouping());

        // 2 Host containers + 2 Port elements
        let containers: Vec<&Node> = nodes
            .iter()
            .filter(|n| {
                matches!(
                    n.node_type,
                    NodeType::Container {
                        container_type: ContainerType::Host,
                        ..
                    }
                )
            })
            .collect();
        assert_eq!(containers.len(), 2);

        let elements: Vec<&Node> = nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Element { .. }))
            .collect();
        assert_eq!(elements.len(), 2);

        // 1 PhysicalLink edge
        assert_eq!(edges.len(), 1);
        assert!(matches!(edges[0].edge_type, EdgeType::PhysicalLink { .. }));
    }

    #[test]
    fn test_virtual_if_types_excluded() {
        let h1 = make_host("switch-1");
        let h2 = make_host("switch-2");

        let ie_eth = make_if_entry(h1.id, 1, 6, None); // ethernet - included
        let ie_lo = make_if_entry(h1.id, 2, 24, None); // loopback - excluded
        let ie_vlan = make_if_entry(h1.id, 3, 135, None); // l2vlan - excluded
        let ie_tun = make_if_entry(h1.id, 4, 131, None); // tunnel - excluded
        let ie2 = make_if_entry(h2.id, 1, 6, None);

        // Create neighbor link
        let mut ie_eth_linked = ie_eth.clone();
        ie_eth_linked.base.neighbor = Some(Neighbor::Interface(ie2.id));

        let hosts = vec![h1, h2];
        let interfaces = vec![ie_eth_linked, ie_lo, ie_vlan, ie_tun, ie2];
        let options = TopologyOptions::default();
        let ctx = TopologyContext::new(
            &hosts,
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &interfaces,
            &[],
            &[],
            &options,
            crate::server::topology::types::views::TopologyView::L3Logical,
        );

        let builder = L2Builder;
        let (nodes, _edges) = builder.build(&ctx, &l2_grouping());

        let elements: Vec<&Node> = nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Element { .. }))
            .collect();
        // h1: only ethernet port (lo, vlan, tunnel excluded)
        // h2: only ethernet port
        assert_eq!(elements.len(), 2);
    }

    #[test]
    fn test_bidirectional_links_deduped() {
        let h1 = make_host("switch-1");
        let h2 = make_host("switch-2");

        let ie1 = make_if_entry(h1.id, 1, 6, None);
        let ie2 = make_if_entry(h2.id, 1, 6, None);

        // Both entries point to each other (bidirectional LLDP)
        let mut ie1_linked = ie1.clone();
        ie1_linked.base.neighbor = Some(Neighbor::Interface(ie2.id));
        let mut ie2_linked = ie2.clone();
        ie2_linked.base.neighbor = Some(Neighbor::Interface(ie1_linked.id));

        let hosts = vec![h1, h2];
        let interfaces = vec![ie1_linked, ie2_linked];
        let options = TopologyOptions::default();
        let ctx = TopologyContext::new(
            &hosts,
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &interfaces,
            &[],
            &[],
            &options,
            crate::server::topology::types::views::TopologyView::L3Logical,
        );

        let builder = L2Builder;
        let (_nodes, edges) = builder.build(&ctx, &l2_grouping());

        // Only 1 edge despite bidirectional discovery
        assert_eq!(edges.len(), 1);
    }

    #[test]
    fn test_host_only_neighbor_renders_synthetic_element() {
        // A switch port resolved its LLDP neighbor down to a Host (chassis MAC
        // matched, but no Interface record existed on the target — e.g. the
        // target is never SNMP if-table-walked, like the daemon's own host).
        let switch = make_host("switch-1");
        let daemon_host = make_host("scanopy-daemon");

        let mut switch_port = make_if_entry(switch.id, 15, 6, None);
        switch_port.base.neighbor = Some(Neighbor::Host(daemon_host.id));

        let hosts = vec![switch, daemon_host.clone()];
        let interfaces = vec![switch_port];
        let options = TopologyOptions::default();
        let ctx = TopologyContext::new(
            &hosts,
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &interfaces,
            &[],
            &[],
            &options,
            crate::server::topology::types::views::TopologyView::L3Logical,
        );

        let builder = L2Builder;
        let (nodes, edges) = builder.build(&ctx, &l2_grouping());

        // 1 PhysicalLink edge, switch port → daemon host (no specific port)
        assert_eq!(edges.len(), 1);
        assert!(matches!(edges[0].edge_type, EdgeType::PhysicalLink { .. }));
        assert_eq!(edges[0].target, daemon_host.id);

        // Both hosts get containers
        let containers: Vec<&Node> = nodes
            .iter()
            .filter(|n| {
                matches!(
                    n.node_type,
                    NodeType::Container {
                        container_type: ContainerType::Host,
                        ..
                    }
                )
            })
            .collect();
        assert_eq!(containers.len(), 2);

        // Daemon host has no real Interface records, but does get a synthetic
        // Host-typed element standing in for the unresolved port, keyed by its
        // own host_id so the edge target above resolves to a real node.
        let daemon_element = nodes
            .iter()
            .find(|n| n.id == daemon_host.id)
            .expect("synthetic Host element for daemon host");
        assert!(matches!(
            daemon_element.node_type,
            NodeType::Element {
                element: ElementEntityType::Host {},
                ..
            }
        ));
    }

    #[test]
    fn test_host_only_neighbor_self_loop_excluded() {
        let h1 = make_host("switch-1");
        let mut port = make_if_entry(h1.id, 1, 6, None);
        port.base.neighbor = Some(Neighbor::Host(h1.id));

        let hosts = vec![h1];
        let interfaces = vec![port];
        let options = TopologyOptions::default();
        let ctx = TopologyContext::new(
            &hosts,
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &interfaces,
            &[],
            &[],
            &options,
            crate::server::topology::types::views::TopologyView::L3Logical,
        );

        let builder = L2Builder;
        let (_nodes, edges) = builder.build(&ctx, &l2_grouping());
        // Self-referential neighbor data never produces a PhysicalLink edge,
        // even though the host still qualifies for a container (same
        // pre-existing behavior as a self-loop Neighbor::Interface).
        assert!(edges.is_empty());
    }
}

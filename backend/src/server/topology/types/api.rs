use crate::server::{
    bindings::r#impl::base::Binding,
    dependencies::r#impl::base::Dependency,
    hosts::r#impl::base::Host,
    interfaces::r#impl::base::Interface,
    ip_addresses::r#impl::base::IPAddress,
    ports::r#impl::base::Port,
    services::r#impl::base::Service,
    subnets::r#impl::base::Subnet,
    tags::r#impl::base::Tag,
    topology::types::{edges::Edge, layout::Ixy, nodes::Node, views::TopologyView},
    vlans::r#impl::base::Vlan,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

/// Bundle of entities + the built graph that feed the topology render, export,
/// and share pipelines.
///
/// Loaded by [`crate::server::topology::service::main::TopologyService::get_topology_data`]
/// for either the live view (`snapshot_id = None`) or a point-in-time snapshot
/// (`snapshot_id = Some(id)`). The per-view `nodes`/`edges` are built on request
/// from these entities + the network's grouping options
/// (`build_all_view_graphs`) — they are not persisted. The frontend selects the
/// active view's slice client-side.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct TopologyData {
    /// Hosts included in this topology.
    pub hosts: Vec<Host>,
    /// IP addresses included in this topology.
    pub ip_addresses: Vec<IPAddress>,
    /// Subnets included in this topology.
    pub subnets: Vec<Subnet>,
    /// Dependencies included in this topology.
    pub dependencies: Vec<Dependency>,
    /// Ports included in this topology.
    pub ports: Vec<Port>,
    /// Service bindings included in this topology.
    pub bindings: Vec<Binding>,
    /// Interfaces included in this topology.
    pub interfaces: Vec<Interface>,
    /// Services included in this topology.
    pub services: Vec<Service>,
    /// VLANs included in this topology.
    pub vlans: Vec<Vlan>,
    /// Tags assigned to this entity.
    pub tags: Vec<Tag>,
    /// Per-view graph built on request from the entities above + grouping
    /// options. Keyed by view so switching the active perspective is a
    /// client-side slice selection.
    #[serde(default)]
    pub nodes: HashMap<TopologyView, Vec<Node>>,
    /// Connections between the nodes of the built graph.
    #[serde(default)]
    pub edges: HashMap<TopologyView, Vec<Edge>>,
    /// Views whose data is present in this entity set (L3/Workloads always;
    /// L2 Physical iff LLDP/CDP neighbors exist; Application iff app-flagged
    /// tags are used). The topology tab restricts a snapshot's view picker to
    /// these — you can't set up SNMP or create app tags on a historical
    /// snapshot — while the live view shows all views with setup prompts.
    #[serde(default)]
    pub available_views: Vec<TopologyView>,
    /// Manual positions for the live topology. Historical snapshots always
    /// return an empty list because layout overrides are mutable presentation
    /// state, not point-in-time discovery data.
    #[serde(default)]
    pub layout_overrides: Vec<TopologyNodePosition>,
    /// Human-readable lines explaining why some L2 neighbour data could not be correlated to a
    /// known host (unresolved LLDP/CDP neighbours, unmatched forwarding-table entries). Always
    /// empty for a historical snapshot — correlation reflects the network's current cumulative
    /// state, not a point-in-time capture. See
    /// upstream's reciprocal LLDP resolution reporting.
    #[serde(default)]
    pub l2_diagnostics: Vec<String>,
}

/// A persisted manual position for one node in one topology view.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct TopologyNodePosition {
    /// The topology this override applies to.
    pub topology_id: uuid::Uuid,
    pub view: TopologyView,
    /// The node this override positions.
    pub node_id: uuid::Uuid,
    /// The node's current derived parent when this position was saved. Clients
    /// ignore an override when this no longer matches the freshly built graph.
    #[schema(value_type = Option<String>, required)]
    pub parent_node_id: Option<uuid::Uuid>,
    pub position: Ixy,
    /// When this override was first saved.
    pub created_at: DateTime<Utc>,
    /// When this override was last saved.
    pub updated_at: DateTime<Utc>,
}

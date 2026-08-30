use crate::server::bindings::r#impl::base::Binding;
use crate::server::categories::r#impl::base::Category;
use crate::server::credentials::r#impl::base::Credential;
use crate::server::custom_service_definitions::r#impl::base::CustomServiceDefinition;
use crate::server::custom_topology_views::r#impl::base::CustomTopologyView;
use crate::server::custom_view_edges::r#impl::base::CustomViewEdge;
use crate::server::custom_view_nodes::r#impl::base::CustomViewNode;
use crate::server::host_images::r#impl::base::HostImage;
use crate::server::interfaces::r#impl::base::Interface;
use crate::server::invites::r#impl::base::Invite;
use crate::server::ip_addresses::r#impl::base::IPAddress;
use crate::server::library_objects::r#impl::base::LibraryObject;
use crate::server::ports::r#impl::base::Port;
use crate::server::services::r#impl::base::Service;
use crate::server::shares::r#impl::base::Share;
use crate::server::snapshots::types::base::Snapshot;
use crate::server::subnets::r#impl::base::Subnet;
use crate::server::topology::types::base::Topology;
use crate::server::vlans::r#impl::base::Vlan;
use crate::server::{dependencies::r#impl::base::Dependency, tags::r#impl::base::Tag};
use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, Display, EnumDiscriminants, EnumIter, IntoStaticStr, VariantNames};
use utoipa::ToSchema;

use crate::server::{
    daemon_api_keys::r#impl::base::DaemonApiKey,
    daemons::r#impl::base::Daemon,
    discovery::r#impl::base::Discovery,
    hosts::r#impl::base::Host,
    networks::r#impl::Network,
    organizations::r#impl::base::Organization,
    shared::{
        storage::traits::Entity as EntityTrait,
        types::{
            Color, Icon,
            metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
        },
    },
    user_api_keys::r#impl::base::UserApiKey,
    users::r#impl::base::User,
};

// Trait use to determine whether a given property change on an entity should trigger a rebuild of topology
pub trait ChangeTriggersTopologyStaleness<T> {
    fn triggers_staleness(&self, _other: Option<T>) -> bool;
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    EnumDiscriminants,
    IntoStaticStr,
    Serialize,
    Deserialize,
    Display,
    Default,
)]
#[strum_discriminants(derive(
    Display,
    AsRefStr,
    Hash,
    EnumIter,
    IntoStaticStr,
    Serialize,
    Deserialize,
    ToSchema,
    Default,
    VariantNames,
))]
pub enum Entity {
    Organization(Organization),
    Invite(Invite),
    Share(Share),
    Network(Network),
    DaemonApiKey(DaemonApiKey),
    UserApiKey(UserApiKey),
    User(User),
    Tag(Tag),

    Discovery(Discovery),
    Daemon(Daemon),

    Host(Host),
    Service(Service),
    Port(Port),
    Binding(Binding),
    IPAddress(IPAddress),
    Interface(Interface),
    HostImage(HostImage),

    Credential(Credential),
    Subnet(Subnet),
    Vlan(Vlan),
    Dependency(Dependency),
    Topology(Box<Topology>),
    Snapshot(Snapshot),
    CustomTopologyView(CustomTopologyView),
    CustomViewNode(CustomViewNode),
    CustomViewEdge(CustomViewEdge),
    LibraryObject(LibraryObject),
    Category(Category),
    CustomServiceDefinition(CustomServiceDefinition),

    #[default]
    #[strum_discriminants(default)]
    Unknown,
}

impl HasId for EntityDiscriminants {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl Entity {
    /// Title-case singular/plural names sourced from each concrete type's
    /// `Entity::ENTITY_NAME_SINGULAR` / `ENTITY_NAME_PLURAL` const. Single
    /// match for all variants; both names in one tuple to avoid duplicating
    /// the enumeration.
    pub fn entity_names(&self) -> (&'static str, &'static str) {
        match self {
            Entity::Organization(_) => (
                <Organization as EntityTrait>::ENTITY_NAME_SINGULAR,
                <Organization as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::Invite(_) => (
                <Invite as EntityTrait>::ENTITY_NAME_SINGULAR,
                <Invite as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::Share(_) => (
                <Share as EntityTrait>::ENTITY_NAME_SINGULAR,
                <Share as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::Network(_) => (
                <Network as EntityTrait>::ENTITY_NAME_SINGULAR,
                <Network as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::DaemonApiKey(_) => (
                <DaemonApiKey as EntityTrait>::ENTITY_NAME_SINGULAR,
                <DaemonApiKey as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::UserApiKey(_) => (
                <UserApiKey as EntityTrait>::ENTITY_NAME_SINGULAR,
                <UserApiKey as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::User(_) => (
                <User as EntityTrait>::ENTITY_NAME_SINGULAR,
                <User as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::Tag(_) => (
                <Tag as EntityTrait>::ENTITY_NAME_SINGULAR,
                <Tag as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::Discovery(_) => (
                <Discovery as EntityTrait>::ENTITY_NAME_SINGULAR,
                <Discovery as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::Daemon(_) => (
                <Daemon as EntityTrait>::ENTITY_NAME_SINGULAR,
                <Daemon as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::Host(_) => (
                <Host as EntityTrait>::ENTITY_NAME_SINGULAR,
                <Host as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::Service(_) => (
                <Service as EntityTrait>::ENTITY_NAME_SINGULAR,
                <Service as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::Port(_) => (
                <Port as EntityTrait>::ENTITY_NAME_SINGULAR,
                <Port as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::Binding(_) => (
                <Binding as EntityTrait>::ENTITY_NAME_SINGULAR,
                <Binding as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::IPAddress(_) => (
                <IPAddress as EntityTrait>::ENTITY_NAME_SINGULAR,
                <IPAddress as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::Interface(_) => (
                <Interface as EntityTrait>::ENTITY_NAME_SINGULAR,
                <Interface as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::HostImage(_) => (
                <HostImage as EntityTrait>::ENTITY_NAME_SINGULAR,
                <HostImage as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::Credential(_) => (
                <Credential as EntityTrait>::ENTITY_NAME_SINGULAR,
                <Credential as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::Subnet(_) => (
                <Subnet as EntityTrait>::ENTITY_NAME_SINGULAR,
                <Subnet as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::Vlan(_) => (
                <Vlan as EntityTrait>::ENTITY_NAME_SINGULAR,
                <Vlan as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::Dependency(_) => (
                <Dependency as EntityTrait>::ENTITY_NAME_SINGULAR,
                <Dependency as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::Topology(_) => (
                <Topology as EntityTrait>::ENTITY_NAME_SINGULAR,
                <Topology as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::Snapshot(_) => (
                <Snapshot as EntityTrait>::ENTITY_NAME_SINGULAR,
                <Snapshot as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::CustomTopologyView(_) => (
                <CustomTopologyView as EntityTrait>::ENTITY_NAME_SINGULAR,
                <CustomTopologyView as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::CustomViewNode(_) => (
                <CustomViewNode as EntityTrait>::ENTITY_NAME_SINGULAR,
                <CustomViewNode as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::CustomViewEdge(_) => (
                <CustomViewEdge as EntityTrait>::ENTITY_NAME_SINGULAR,
                <CustomViewEdge as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::LibraryObject(_) => (
                <LibraryObject as EntityTrait>::ENTITY_NAME_SINGULAR,
                <LibraryObject as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::Category(_) => (
                <Category as EntityTrait>::ENTITY_NAME_SINGULAR,
                <Category as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::CustomServiceDefinition(_) => (
                <CustomServiceDefinition as EntityTrait>::ENTITY_NAME_SINGULAR,
                <CustomServiceDefinition as EntityTrait>::ENTITY_NAME_PLURAL,
            ),
            Entity::Unknown => ("Entity", "Entities"),
        }
    }
}

impl EntityDiscriminants {
    /// Title-case singular name, e.g. "Host", "IP Address". Delegates to
    /// `Entity::entity_names` via the existing `From<EntityDiscriminants> for Entity`.
    pub fn entity_name_singular(&self) -> &'static str {
        Entity::from(*self).entity_names().0
    }

    /// Title-case plural name, e.g. "Hosts", "IP Addresses".
    pub fn entity_name_plural(&self) -> &'static str {
        Entity::from(*self).entity_names().1
    }
}

impl EntityDiscriminants {
    /// Whether this entity type supports being tagged directly.
    /// Exhaustive match — adding a new variant forces a decision.
    pub fn is_taggable(&self) -> bool {
        match self {
            EntityDiscriminants::Host
            | EntityDiscriminants::Service
            | EntityDiscriminants::Subnet
            | EntityDiscriminants::Dependency
            | EntityDiscriminants::Network
            | EntityDiscriminants::Discovery
            | EntityDiscriminants::Daemon
            | EntityDiscriminants::DaemonApiKey
            | EntityDiscriminants::UserApiKey
            | EntityDiscriminants::Credential => true,
            EntityDiscriminants::Organization
            | EntityDiscriminants::Invite
            | EntityDiscriminants::Share
            | EntityDiscriminants::User
            | EntityDiscriminants::Tag
            | EntityDiscriminants::Port
            | EntityDiscriminants::Binding
            | EntityDiscriminants::IPAddress
            | EntityDiscriminants::Interface
            | EntityDiscriminants::Vlan
            | EntityDiscriminants::Topology
            | EntityDiscriminants::Snapshot
            | EntityDiscriminants::HostImage
            | EntityDiscriminants::CustomTopologyView
            | EntityDiscriminants::CustomViewNode
            | EntityDiscriminants::CustomViewEdge
            | EntityDiscriminants::LibraryObject
            | EntityDiscriminants::Category
            | EntityDiscriminants::CustomServiceDefinition
            | EntityDiscriminants::Unknown => false,
        }
    }

    /// The nearest taggable ancestor of this entity — used to resolve which
    /// entity's tags apply when a non-taggable entity is involved in tag-based
    /// rules, filters, or selections (e.g. IP addresses/interfaces/ports
    /// resolve to their owning Host).
    ///
    /// Returns `None` when the entity is itself taggable (see `is_taggable`)
    /// or when no taggable ancestor exists.
    pub fn parent_taggable_entity(&self) -> Option<EntityDiscriminants> {
        match self {
            EntityDiscriminants::Interface => Some(EntityDiscriminants::Host),
            EntityDiscriminants::IPAddress => Some(EntityDiscriminants::Host),
            EntityDiscriminants::Port => Some(EntityDiscriminants::Host),
            EntityDiscriminants::HostImage => Some(EntityDiscriminants::Host),
            EntityDiscriminants::Service
            | EntityDiscriminants::Binding
            | EntityDiscriminants::Organization
            | EntityDiscriminants::Network
            | EntityDiscriminants::User
            | EntityDiscriminants::Invite
            | EntityDiscriminants::Share
            | EntityDiscriminants::Tag
            | EntityDiscriminants::DaemonApiKey
            | EntityDiscriminants::UserApiKey
            | EntityDiscriminants::Daemon
            | EntityDiscriminants::Discovery
            | EntityDiscriminants::Credential
            | EntityDiscriminants::Host
            | EntityDiscriminants::Subnet
            | EntityDiscriminants::Vlan
            | EntityDiscriminants::Dependency
            | EntityDiscriminants::Topology
            | EntityDiscriminants::Snapshot
            | EntityDiscriminants::CustomTopologyView
            | EntityDiscriminants::CustomViewNode
            | EntityDiscriminants::CustomViewEdge
            | EntityDiscriminants::LibraryObject
            | EntityDiscriminants::Category
            | EntityDiscriminants::CustomServiceDefinition
            | EntityDiscriminants::Unknown => None,
        }
    }
}

impl EntityMetadataProvider for EntityDiscriminants {
    fn color(&self) -> Color {
        match self {
            EntityDiscriminants::Organization => Color::Blue,
            EntityDiscriminants::Network => Color::Blue,
            EntityDiscriminants::User => Color::Blue,
            EntityDiscriminants::Invite => Color::Sky,

            EntityDiscriminants::Tag => Color::Yellow,

            EntityDiscriminants::Daemon => Color::Green,
            EntityDiscriminants::Discovery => Color::Green,

            EntityDiscriminants::DaemonApiKey => Color::Yellow,
            EntityDiscriminants::UserApiKey => Color::Yellow,
            EntityDiscriminants::Credential => Color::Yellow,

            EntityDiscriminants::Topology => Color::Pink,
            EntityDiscriminants::Snapshot => Color::Purple,
            EntityDiscriminants::Share => Color::Pink,

            EntityDiscriminants::Dependency => Color::Rose,
            EntityDiscriminants::Service => Color::Fuchsia,

            EntityDiscriminants::Host => Color::Amber,

            EntityDiscriminants::Interface => Color::Teal,
            EntityDiscriminants::IPAddress => Color::Emerald,
            EntityDiscriminants::Port => Color::Sky,
            EntityDiscriminants::Binding => Color::Cyan,

            EntityDiscriminants::Subnet => Color::Indigo,
            EntityDiscriminants::Vlan => Color::Violet,
            EntityDiscriminants::HostImage => Color::Amber,

            EntityDiscriminants::CustomTopologyView => Color::Pink,
            EntityDiscriminants::CustomViewNode => Color::Orange,
            EntityDiscriminants::CustomViewEdge => Color::Rose,
            EntityDiscriminants::LibraryObject => Color::Gray,
            EntityDiscriminants::Category => Color::Amber,
            EntityDiscriminants::CustomServiceDefinition => Color::Purple,

            EntityDiscriminants::Unknown => Color::Gray,
        }
    }

    fn icon(&self) -> Icon {
        match self {
            EntityDiscriminants::Organization => Icon::Building,
            EntityDiscriminants::Network => Icon::LandPlot,
            EntityDiscriminants::User => Icon::User,
            EntityDiscriminants::Tag => Icon::Tag,
            EntityDiscriminants::Invite => Icon::UserPlus,
            EntityDiscriminants::Share => Icon::Share2,
            EntityDiscriminants::DaemonApiKey => Icon::Key,
            EntityDiscriminants::UserApiKey => Icon::Key,
            EntityDiscriminants::Daemon => Icon::SatelliteDish,
            EntityDiscriminants::Discovery => Icon::Radar,
            EntityDiscriminants::Host => Icon::Server,
            EntityDiscriminants::Service => Icon::Layers,
            EntityDiscriminants::IPAddress => Icon::MapPin,
            EntityDiscriminants::Port => Icon::Binary,
            EntityDiscriminants::Binding => Icon::Link,
            EntityDiscriminants::Interface => Icon::EthernetPort,
            EntityDiscriminants::Credential => Icon::Asterisk,
            EntityDiscriminants::Subnet => Icon::Cloud,
            EntityDiscriminants::Vlan => Icon::Network,
            EntityDiscriminants::Dependency => Icon::Waypoints,
            EntityDiscriminants::Topology => Icon::ChartBarStacked,
            EntityDiscriminants::Snapshot => Icon::Camera,
            EntityDiscriminants::HostImage => Icon::Image,

            EntityDiscriminants::CustomTopologyView => Icon::PenTool,
            EntityDiscriminants::CustomViewNode => Icon::Shapes,
            EntityDiscriminants::CustomViewEdge => Icon::Spline,
            EntityDiscriminants::LibraryObject => Icon::LayoutGrid,
            EntityDiscriminants::Category => Icon::Tags,
            EntityDiscriminants::CustomServiceDefinition => Icon::Boxes,

            EntityDiscriminants::Unknown => Icon::CircleQuestionMark,
        }
    }
}

impl TypeMetadataProvider for EntityDiscriminants {
    fn name(&self) -> &'static str {
        self.into()
    }

    fn metadata(&self) -> serde_json::Value {
        let mut m = serde_json::Map::new();
        if let Some(parent) = self.parent_taggable_entity() {
            m.insert(
                "parent_taggable_entity".to_string(),
                serde_json::json!(parent),
            );
        }
        m.insert(
            "is_taggable".to_string(),
            serde_json::json!(self.is_taggable()),
        );
        m.insert(
            "entity_name_singular".to_string(),
            serde_json::json!(self.entity_name_singular()),
        );
        m.insert(
            "entity_name_plural".to_string(),
            serde_json::json!(self.entity_name_plural()),
        );
        serde_json::Value::Object(m)
    }
}

impl From<Organization> for Entity {
    fn from(value: Organization) -> Self {
        Self::Organization(value)
    }
}

impl From<Invite> for Entity {
    fn from(value: Invite) -> Self {
        Self::Invite(value)
    }
}

impl From<Share> for Entity {
    fn from(value: Share) -> Self {
        Self::Share(value)
    }
}

impl From<Network> for Entity {
    fn from(value: Network) -> Self {
        Self::Network(value)
    }
}

impl From<DaemonApiKey> for Entity {
    fn from(value: DaemonApiKey) -> Self {
        Self::DaemonApiKey(value)
    }
}

impl From<UserApiKey> for Entity {
    fn from(value: UserApiKey) -> Self {
        Self::UserApiKey(value)
    }
}

impl From<User> for Entity {
    fn from(value: User) -> Self {
        Self::User(value)
    }
}

impl From<Discovery> for Entity {
    fn from(value: Discovery) -> Self {
        Self::Discovery(value)
    }
}

impl From<Daemon> for Entity {
    fn from(value: Daemon) -> Self {
        Self::Daemon(value)
    }
}

impl From<Host> for Entity {
    fn from(value: Host) -> Self {
        Self::Host(value)
    }
}

impl From<Service> for Entity {
    fn from(value: Service) -> Self {
        Self::Service(value)
    }
}

impl From<Port> for Entity {
    fn from(value: Port) -> Self {
        Self::Port(value)
    }
}

impl From<Binding> for Entity {
    fn from(value: Binding) -> Self {
        Self::Binding(value)
    }
}

impl From<IPAddress> for Entity {
    fn from(value: IPAddress) -> Self {
        Self::IPAddress(value)
    }
}

impl From<Subnet> for Entity {
    fn from(value: Subnet) -> Self {
        Self::Subnet(value)
    }
}

impl From<Vlan> for Entity {
    fn from(value: Vlan) -> Self {
        Self::Vlan(value)
    }
}

impl From<Dependency> for Entity {
    fn from(value: Dependency) -> Self {
        Self::Dependency(value)
    }
}

impl From<Topology> for Entity {
    fn from(value: Topology) -> Self {
        Self::Topology(Box::new(value))
    }
}

impl From<Snapshot> for Entity {
    fn from(value: Snapshot) -> Self {
        Self::Snapshot(value)
    }
}

impl From<Tag> for Entity {
    fn from(value: Tag) -> Self {
        Self::Tag(value)
    }
}

impl From<Credential> for Entity {
    fn from(value: Credential) -> Self {
        Self::Credential(value)
    }
}

impl From<Interface> for Entity {
    fn from(value: Interface) -> Self {
        Self::Interface(value)
    }
}

impl From<HostImage> for Entity {
    fn from(value: HostImage) -> Self {
        Self::HostImage(value)
    }
}

impl From<CustomTopologyView> for Entity {
    fn from(value: CustomTopologyView) -> Self {
        Self::CustomTopologyView(value)
    }
}

impl From<CustomViewNode> for Entity {
    fn from(value: CustomViewNode) -> Self {
        Self::CustomViewNode(value)
    }
}

impl From<CustomViewEdge> for Entity {
    fn from(value: CustomViewEdge) -> Self {
        Self::CustomViewEdge(value)
    }
}

impl From<LibraryObject> for Entity {
    fn from(value: LibraryObject) -> Self {
        Self::LibraryObject(value)
    }
}

impl From<Category> for Entity {
    fn from(value: Category) -> Self {
        Self::Category(value)
    }
}

impl From<CustomServiceDefinition> for Entity {
    fn from(value: CustomServiceDefinition) -> Self {
        Self::CustomServiceDefinition(value)
    }
}

impl From<EntityDiscriminants> for Entity {
    fn from(d: EntityDiscriminants) -> Self {
        match d {
            EntityDiscriminants::Host => Entity::Host(Host::default()),
            EntityDiscriminants::Service => Entity::Service(Service::default()),
            EntityDiscriminants::Subnet => Entity::Subnet(Subnet::default()),
            EntityDiscriminants::Vlan => Entity::Vlan(Vlan::default()),
            EntityDiscriminants::Dependency => Entity::Dependency(Dependency::default()),
            EntityDiscriminants::Port => Entity::Port(Port::default()),
            EntityDiscriminants::IPAddress => Entity::IPAddress(IPAddress::default()),
            EntityDiscriminants::Binding => Entity::Binding(Binding::default()),
            EntityDiscriminants::Interface => Entity::Interface(Interface::default()),
            EntityDiscriminants::HostImage => Entity::HostImage(HostImage::default()),
            EntityDiscriminants::Tag => Entity::Tag(Tag::default()),
            EntityDiscriminants::Network => Entity::Network(Network::default()),
            EntityDiscriminants::Organization => Entity::Organization(Organization::default()),
            EntityDiscriminants::User => Entity::User(User::default()),
            EntityDiscriminants::Invite => Entity::Invite(Invite::default()),
            EntityDiscriminants::Share => Entity::Share(Share::default()),
            EntityDiscriminants::Discovery => Entity::Discovery(Discovery::default()),
            EntityDiscriminants::Daemon => Entity::Daemon(Daemon::default()),
            EntityDiscriminants::DaemonApiKey => Entity::DaemonApiKey(DaemonApiKey::default()),
            EntityDiscriminants::UserApiKey => Entity::UserApiKey(UserApiKey::default()),
            EntityDiscriminants::Credential => Entity::Credential(Credential::default()),
            EntityDiscriminants::Topology => Entity::Topology(Box::default()),
            EntityDiscriminants::Snapshot => Entity::Snapshot(Snapshot::default()),
            EntityDiscriminants::CustomTopologyView => {
                Entity::CustomTopologyView(CustomTopologyView::default())
            }
            EntityDiscriminants::CustomViewNode => {
                Entity::CustomViewNode(CustomViewNode::default())
            }
            EntityDiscriminants::CustomViewEdge => {
                Entity::CustomViewEdge(CustomViewEdge::default())
            }
            EntityDiscriminants::LibraryObject => Entity::LibraryObject(LibraryObject::default()),
            EntityDiscriminants::Category => Entity::Category(Category::default()),
            EntityDiscriminants::CustomServiceDefinition => {
                Entity::CustomServiceDefinition(CustomServiceDefinition::default())
            }
            EntityDiscriminants::Unknown => Entity::Unknown,
        }
    }
}

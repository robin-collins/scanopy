use crate::server::shared::entities::ChangeTriggersTopologyStaleness;
use crate::server::shared::position::Positioned;
use crate::server::subnets::r#impl::base::Subnet;
use chrono::{DateTime, Utc};
use mac_address::MacAddress;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::hash::Hash;
use std::net::{IpAddr, Ipv4Addr};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

pub const ALL_INTERFACES_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, ToSchema, Validate)]
pub struct InterfaceBase {
    pub network_id: Uuid,
    pub host_id: Uuid,
    pub subnet_id: Uuid,
    #[schema(value_type = String)]
    pub ip_address: IpAddr,
    /// MAC address discovered from ARP, SNMP, or Docker - immutable once set
    #[schema(value_type = Option<String>)]
    pub mac_address: Option<MacAddress>,
    #[schema(required)]
    pub name: Option<String>,
    /// Position of this interface in the host's interface list (for ordering)
    #[serde(default)]
    pub position: i32,
}

impl Default for InterfaceBase {
    fn default() -> Self {
        Self {
            network_id: Uuid::nil(),
            host_id: Uuid::nil(),
            subnet_id: Uuid::nil(),
            ip_address: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            mac_address: None,
            name: None,
            position: 0,
        }
    }
}

impl InterfaceBase {
    /// Create a conceptual interface for a subnet.
    /// `host_id` can be `Uuid::nil()` as a placeholder - server will set the correct one.
    pub fn new_conceptual(host_id: Uuid, subnet: &Subnet) -> Self {
        let ip_address = IpAddr::V4(Ipv4Addr::new(203, 0, 113, rand::rng().random_range(1..255)));

        Self {
            network_id: subnet.base.network_id,
            host_id,
            subnet_id: subnet.id,
            ip_address,
            mac_address: None,
            name: Some(subnet.base.name.clone()),
            position: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, Default, ToSchema, Validate)]
#[schema(example = crate::server::shared::types::examples::interface)]
pub struct Interface {
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
    pub base: InterfaceBase,
}

impl Hash for Interface {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.base.ip_address.hash(state);
        self.base.subnet_id.hash(state);
    }
}

/// Two interfaces are equal when they represent the same logical interface:
/// - Same IP address on the same subnet (primary network identity), OR
/// - Same database ID (both non-nil)
///
/// MAC address is intentionally excluded from equality. VLAN sub-interfaces, bridge
/// members, and bond interfaces legitimately share a parent's MAC while being distinct
/// interfaces with different IPs/subnets. MAC-based matching was previously here (added
/// in 3f69301b for Docker DHCP dedup) but caused VLAN sub-interfaces to collapse into one.
///
/// MAC-based matching now lives in explicit call-site logic where the context allows
/// distinguishing shared MACs (VLANs) from unique MACs (Docker/DHCP):
/// - Interface upsert: `create_with_children()` in `hosts/service.rs`
/// - Host dedup: `find_matching_host_by_interfaces()` in `hosts/service.rs`
/// - Host merge: `merge_hosts()` in `hosts/service.rs`
impl PartialEq for Interface {
    fn eq(&self, other: &Self) -> bool {
        (self.base.ip_address == other.base.ip_address
            && self.base.subnet_id == other.base.subnet_id)
            || (self.id == other.id && self.id != Uuid::nil() && other.id != Uuid::nil())
    }
}

impl Display for Interface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Interface {}: {} on subnet {}",
            self.id, self.base.ip_address, self.base.subnet_id
        )
    }
}

impl Interface {
    pub fn new(base: InterfaceBase) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base,
        }
    }
}

impl ChangeTriggersTopologyStaleness<Interface> for Interface {
    fn triggers_staleness(&self, other: Option<Interface>) -> bool {
        if let Some(other_interface) = other {
            self.base.ip_address != other_interface.base.ip_address
                || self.base.subnet_id != other_interface.base.subnet_id
                || self.base.host_id != other_interface.base.host_id
        } else {
            true
        }
    }
}

impl Positioned for Interface {
    fn position(&self) -> i32 {
        self.base.position
    }

    fn set_position(&mut self, position: i32) {
        self.base.position = position;
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn entity_name() -> &'static str {
        "interface"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn make_interface(ip: IpAddr, subnet_id: Uuid, mac: Option<MacAddress>, id: Uuid) -> Interface {
        Interface {
            id,
            base: InterfaceBase {
                ip_address: ip,
                subnet_id,
                mac_address: mac,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn hash_of(iface: &Interface) -> u64 {
        let mut hasher = DefaultHasher::new();
        iface.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn same_mac_different_ip_subnet_not_equal() {
        let mac = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x01]);
        let s1 = Uuid::new_v4();
        let s2 = Uuid::new_v4();
        let a = make_interface("10.0.0.1".parse().unwrap(), s1, Some(mac), Uuid::nil());
        let b = make_interface("20.0.0.1".parse().unwrap(), s2, Some(mac), Uuid::nil());
        assert_ne!(a, b, "VLAN sub-interfaces with same MAC must not be equal");
    }

    #[test]
    fn same_ip_subnet_equal_regardless_of_mac() {
        let subnet = Uuid::new_v4();
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        let mac_a = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x01]);
        let mac_b = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x02]);
        let a = make_interface(ip, subnet, Some(mac_a), Uuid::nil());
        let b = make_interface(ip, subnet, Some(mac_b), Uuid::nil());
        assert_eq!(a, b, "Same IP+subnet should be equal regardless of MAC");
    }

    #[test]
    fn hash_consistent_with_eq() {
        let subnet = Uuid::new_v4();
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        let mac_a = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x01]);
        let mac_b = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x02]);
        let a = make_interface(ip, subnet, Some(mac_a), Uuid::nil());
        let b = make_interface(ip, subnet, Some(mac_b), Uuid::nil());
        assert_eq!(a, b);
        assert_eq!(
            hash_of(&a),
            hash_of(&b),
            "Equal interfaces must have equal hashes"
        );
    }

    #[test]
    fn nil_ids_not_equal_when_different_ip_subnet() {
        let s1 = Uuid::new_v4();
        let s2 = Uuid::new_v4();
        let a = make_interface("10.0.0.1".parse().unwrap(), s1, None, Uuid::nil());
        let b = make_interface("20.0.0.1".parse().unwrap(), s2, None, Uuid::nil());
        assert_ne!(a, b, "Nil IDs with different IP/subnet must not be equal");
    }

    #[test]
    fn same_non_nil_id_equal() {
        let id = Uuid::new_v4();
        let s1 = Uuid::new_v4();
        let s2 = Uuid::new_v4();
        let a = make_interface("10.0.0.1".parse().unwrap(), s1, None, id);
        let b = make_interface("20.0.0.1".parse().unwrap(), s2, None, id);
        assert_eq!(a, b, "Same non-nil ID should be equal");
    }
}

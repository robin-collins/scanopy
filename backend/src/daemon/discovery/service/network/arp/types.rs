use mac_address::MacAddress;
use std::net::Ipv4Addr;

/// Result of ARP scanning a single host
#[derive(Debug, Clone)]
pub struct ArpScanResult {
    pub ip: Ipv4Addr,
    pub mac: MacAddress,
}

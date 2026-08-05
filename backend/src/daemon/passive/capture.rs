use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use chrono::{Duration as ChronoDuration, Utc};
use pnet::{
    datalink::{self, Channel, NetworkInterface},
    packet::{
        Packet,
        arp::{ArpOperations, ArpPacket},
        ethernet::{EtherTypes, EthernetPacket},
        ip::IpNextHeaderProtocols,
        ipv4::{Ipv4Flags, Ipv4Packet},
        ipv6::Ipv6Packet,
        udp::UdpPacket,
        vlan::VlanPacket,
    },
};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::server::passive::types::{
    NeighborState, PassiveFact, PassiveObservationInput, PassiveSource,
};

use super::parsers::{parse_dhcp, parse_mdns};

pub fn spawn_capture_tasks(
    selected_interfaces: &[String],
    sender: mpsc::Sender<PassiveObservationInput>,
    running: Arc<AtomicBool>,
) {
    for interface in datalink::interfaces().into_iter().filter(|interface| {
        interface.is_up()
            && !interface.is_loopback()
            && (selected_interfaces.is_empty()
                || selected_interfaces.iter().any(|selected| {
                    let selected_address = selected.split('/').next().unwrap_or(selected);
                    selected == &interface.name
                        || interface
                            .ips
                            .iter()
                            .any(|ip| ip.ip().to_string() == selected_address)
                }))
    }) {
        let task_sender = sender.clone();
        let task_running = running.clone();
        tokio::task::spawn_blocking(move || {
            capture_interface(interface, task_sender, task_running)
        });
    }
}

fn capture_interface(
    interface: NetworkInterface,
    sender: mpsc::Sender<PassiveObservationInput>,
    running: Arc<AtomicBool>,
) {
    let config = datalink::Config {
        read_timeout: Some(Duration::from_secs(1)),
        promiscuous: false,
        read_buffer_size: 65_536,
        ..Default::default()
    };
    let Ok(Channel::Ethernet(_, mut receiver)) = datalink::channel(&interface, config) else {
        tracing::warn!(interface = %interface.name, "Passive capture unavailable; continuing without this interface");
        return;
    };
    tracing::info!(interface = %interface.name, "Passive metadata capture started");
    while running.load(Ordering::Relaxed) {
        let Ok(frame) = receiver.next() else {
            continue;
        };
        let Some(ethernet) = EthernetPacket::new(frame) else {
            continue;
        };
        let observations = parse_ether_payload(
            ethernet.get_ethertype(),
            ethernet.payload(),
            &interface.name,
            0,
        );
        for observation in observations {
            let _ = sender.try_send(observation);
        }
    }
}

fn parse_ether_payload(
    ethertype: pnet::packet::ethernet::EtherType,
    payload: &[u8],
    interface: &str,
    vlan_depth: u8,
) -> Vec<PassiveObservationInput> {
    match ethertype {
        EtherTypes::Arp => parse_arp(payload, interface).into_iter().collect(),
        EtherTypes::Ipv4 => parse_ipv4_udp(payload),
        EtherTypes::Ipv6 => parse_ipv6_udp(payload),
        EtherTypes::Vlan | EtherTypes::PBridge | EtherTypes::QinQ if vlan_depth < 2 => {
            let Some(vlan) = VlanPacket::new(payload) else {
                return vec![];
            };
            parse_ether_payload(
                vlan.get_ethertype(),
                vlan.payload(),
                interface,
                vlan_depth + 1,
            )
        }
        _ => vec![],
    }
}

fn parse_ipv4_udp(payload: &[u8]) -> Vec<PassiveObservationInput> {
    let Some(ipv4) = Ipv4Packet::new(payload) else {
        return vec![];
    };
    if ipv4.get_next_level_protocol() != IpNextHeaderProtocols::Udp
        || ipv4.get_fragment_offset() != 0
        || ipv4.get_flags() & Ipv4Flags::MoreFragments != 0
    {
        return vec![];
    }
    parse_udp_payload(ipv4.payload())
}

fn parse_ipv6_udp(payload: &[u8]) -> Vec<PassiveObservationInput> {
    let Some(ipv6) = Ipv6Packet::new(payload) else {
        return vec![];
    };
    // Extension headers, including fragments, are deliberately not traversed.
    // Only a direct UDP next-header is accepted, preventing fragment confusion.
    if ipv6.get_next_header() != IpNextHeaderProtocols::Udp {
        return vec![];
    }
    parse_udp_payload(ipv6.payload())
}

fn parse_udp_payload(payload: &[u8]) -> Vec<PassiveObservationInput> {
    let Some(udp) = UdpPacket::new(payload) else {
        return vec![];
    };
    let source = udp.get_source();
    let destination = udp.get_destination();
    if source == 5353 || destination == 5353 {
        return parse_mdns(udp.payload()).unwrap_or_default();
    }
    if matches!((source, destination), (67, 68) | (68, 67)) {
        return parse_dhcp(udp.payload()).into_iter().collect();
    }
    vec![]
}

fn parse_arp(payload: &[u8], interface: &str) -> Option<PassiveObservationInput> {
    let arp = ArpPacket::new(payload)?;
    if !matches!(
        arp.get_operation(),
        ArpOperations::Request | ArpOperations::Reply
    ) {
        return None;
    }
    let address = arp.get_sender_proto_addr();
    if address.is_unspecified() {
        return None;
    }
    let observed_at = Utc::now();
    Some(PassiveObservationInput {
        observation_id: Uuid::new_v4(),
        source: PassiveSource::Arp,
        confidence: 70,
        observed_at,
        expires_at: Some(observed_at + ChronoDuration::minutes(30)),
        fact: PassiveFact::NeighborMapping {
            address: address.into(),
            mac_address: Some(arp.get_sender_hw_addr().octets().into()),
            interface: interface.chars().take(255).collect(),
            state: NeighborState::Reachable,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnet::packet::ethernet::EtherTypes;
    use pnet::packet::{
        arp::{ArpHardwareTypes, MutableArpPacket},
        vlan::MutableVlanPacket,
    };
    use pnet::util::MacAddr;

    #[test]
    fn arp_parser_keeps_only_sender_metadata() {
        let mut bytes = [0u8; 28];
        let mut packet = MutableArpPacket::new(&mut bytes).unwrap();
        packet.set_hardware_type(ArpHardwareTypes::Ethernet);
        packet.set_protocol_type(EtherTypes::Ipv4);
        packet.set_hw_addr_len(6);
        packet.set_proto_addr_len(4);
        packet.set_operation(ArpOperations::Reply);
        packet.set_sender_hw_addr(MacAddr::new(2, 0, 0, 0, 0, 1));
        packet.set_sender_proto_addr("192.0.2.10".parse().unwrap());
        let observation = parse_arp(&bytes, "eth0").unwrap();
        assert!(matches!(
            observation.fact,
            PassiveFact::NeighborMapping { .. }
        ));
    }

    #[test]
    fn ipv4_fragments_are_rejected_before_udp_parsing() {
        let mut bytes = [0u8; 28];
        bytes[0] = 0x45;
        bytes[6] = 0x20; // More Fragments.
        bytes[9] = IpNextHeaderProtocols::Udp.0;
        bytes[20..22].copy_from_slice(&5353u16.to_be_bytes());
        bytes[22..24].copy_from_slice(&5353u16.to_be_bytes());
        assert!(parse_ipv4_udp(&bytes).is_empty());

        bytes[6] = 0;
        bytes[7] = 1; // Non-zero fragment offset.
        assert!(parse_ipv4_udp(&bytes).is_empty());
    }

    #[test]
    fn vlan_arp_uses_the_same_bounded_parser() {
        let mut arp_bytes = [0u8; 28];
        let mut arp = MutableArpPacket::new(&mut arp_bytes).unwrap();
        arp.set_hardware_type(ArpHardwareTypes::Ethernet);
        arp.set_protocol_type(EtherTypes::Ipv4);
        arp.set_hw_addr_len(6);
        arp.set_proto_addr_len(4);
        arp.set_operation(ArpOperations::Reply);
        arp.set_sender_hw_addr(MacAddr::new(2, 0, 0, 0, 0, 2));
        arp.set_sender_proto_addr("192.0.2.11".parse().unwrap());

        let mut vlan_bytes = vec![0u8; 4 + arp_bytes.len()];
        let mut vlan = MutableVlanPacket::new(&mut vlan_bytes).unwrap();
        vlan.set_ethertype(EtherTypes::Arp);
        vlan.set_payload(&arp_bytes);

        assert_eq!(
            parse_ether_payload(EtherTypes::Vlan, &vlan_bytes, "eth0", 0).len(),
            1
        );
    }
}

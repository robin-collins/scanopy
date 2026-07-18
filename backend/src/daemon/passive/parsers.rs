#[cfg(any(target_os = "linux", test))]
use std::str::FromStr;
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use anyhow::{anyhow, bail};
use chrono::{Duration, Utc};
use mac_address::MacAddress;
use uuid::Uuid;

#[cfg(any(target_os = "linux", test))]
use crate::server::passive::types::NeighborState;
use crate::server::passive::types::{
    DhcpMessageType, PassiveFact, PassiveObservationInput, PassiveSource,
};

const MAX_MDNS_BYTES: usize = 9_000;
const MAX_DNS_RECORDS: usize = 128;
const MAX_DHCP_BYTES: usize = 4_096;

#[derive(Default)]
struct MdnsRecords {
    ptrs: Vec<(String, String, u32)>,
    srv: HashMap<String, (String, u16, u32)>,
    txt: HashMap<String, Vec<String>>,
    addresses: HashMap<String, Vec<IpAddr>>,
}

pub fn parse_mdns(payload: &[u8]) -> anyhow::Result<Vec<PassiveObservationInput>> {
    if payload.len() < 12 || payload.len() > MAX_MDNS_BYTES {
        bail!("mDNS payload is outside parser bounds");
    }
    let flags = u16::from_be_bytes([payload[2], payload[3]]);
    if flags & 0x8000 == 0 {
        return Ok(vec![]); // queries do not assert inventory facts
    }
    let questions = usize::from(u16::from_be_bytes([payload[4], payload[5]]));
    let record_count = usize::from(u16::from_be_bytes([payload[6], payload[7]]))
        + usize::from(u16::from_be_bytes([payload[8], payload[9]]))
        + usize::from(u16::from_be_bytes([payload[10], payload[11]]));
    if questions > MAX_DNS_RECORDS || record_count > MAX_DNS_RECORDS {
        bail!("mDNS record count exceeds parser bound");
    }
    let mut offset = 12;
    for _ in 0..questions {
        let (_, next) = dns_name(payload, offset)?;
        offset = next
            .checked_add(4)
            .ok_or_else(|| anyhow!("mDNS offset overflow"))?;
        if offset > payload.len() {
            bail!("mDNS question is truncated");
        }
    }

    let mut records = MdnsRecords::default();
    for _ in 0..record_count {
        let (owner, next) = dns_name(payload, offset)?;
        offset = next;
        if offset + 10 > payload.len() {
            bail!("mDNS resource record is truncated");
        }
        let rr_type = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
        let ttl = u32::from_be_bytes(payload[offset + 4..offset + 8].try_into().unwrap());
        let length = usize::from(u16::from_be_bytes([
            payload[offset + 8],
            payload[offset + 9],
        ]));
        let data_offset = offset + 10;
        let end = data_offset
            .checked_add(length)
            .ok_or_else(|| anyhow!("mDNS record length overflow"))?;
        if end > payload.len() {
            bail!("mDNS resource data is truncated");
        }
        match rr_type {
            1 if length == 4 => {
                let address = IpAddr::V4(Ipv4Addr::new(
                    payload[data_offset],
                    payload[data_offset + 1],
                    payload[data_offset + 2],
                    payload[data_offset + 3],
                ));
                records.addresses.entry(owner).or_default().push(address);
            }
            12 => {
                let (target, consumed) = dns_name(payload, data_offset)?;
                if consumed > end {
                    bail!("mDNS PTR name exceeds its resource record");
                }
                if (owner.contains("._tcp.") || owner.contains("._udp."))
                    && !owner.starts_with("_services._dns-sd")
                {
                    records.ptrs.push((owner, target, ttl));
                }
            }
            16 => {
                let mut cursor = data_offset;
                let mut keys = Vec::new();
                while cursor < end && keys.len() < 32 {
                    let item_len = usize::from(payload[cursor]);
                    cursor += 1;
                    if cursor + item_len > end {
                        bail!("mDNS TXT item is truncated");
                    }
                    let item = &payload[cursor..cursor + item_len];
                    let key_bytes = item.split(|byte| *byte == b'=').next().unwrap_or_default();
                    if !key_bytes.is_empty() {
                        let key = String::from_utf8_lossy(key_bytes).trim().to_string();
                        if !key.is_empty() && key.len() <= 255 && !key.chars().any(char::is_control)
                        {
                            keys.push(key);
                        }
                    }
                    cursor += item_len;
                }
                records.txt.insert(owner, keys);
            }
            28 if length == 16 => {
                let address = IpAddr::V6(Ipv6Addr::from(
                    <[u8; 16]>::try_from(&payload[data_offset..end]).unwrap(),
                ));
                records.addresses.entry(owner).or_default().push(address);
            }
            33 if length >= 6 => {
                let port = u16::from_be_bytes([payload[data_offset + 4], payload[data_offset + 5]]);
                let (hostname, consumed) = dns_name(payload, data_offset + 6)?;
                if consumed > end {
                    bail!("mDNS SRV name exceeds its resource record");
                }
                records.srv.insert(owner, (hostname, port, ttl));
            }
            _ => {}
        }
        offset = end;
    }

    let observed_at = Utc::now();
    let mut output = Vec::new();
    for (service_type, instance, ptr_ttl) in records.ptrs.into_iter().take(64) {
        let (hostname, port, srv_ttl) = records
            .srv
            .get(&instance)
            .map(|(host, port, ttl)| (Some(host.clone()), Some(*port), *ttl))
            .unwrap_or((None, None, ptr_ttl));
        let mut addresses = hostname
            .as_ref()
            .and_then(|host| records.addresses.get(host))
            .cloned()
            .unwrap_or_default();
        addresses.sort();
        addresses.dedup();
        addresses.truncate(16);
        let ttl_seconds = ptr_ttl.min(srv_ttl).min(86_400);
        output.push(PassiveObservationInput {
            observation_id: Uuid::new_v4(),
            source: PassiveSource::Mdns,
            confidence: 75,
            observed_at,
            expires_at: Some(observed_at + Duration::seconds(i64::from(ttl_seconds))),
            fact: PassiveFact::MdnsService {
                service_type,
                instance: instance.clone(),
                hostname,
                addresses,
                port,
                txt_keys: records.txt.get(&instance).cloned().unwrap_or_default(),
                ttl_seconds,
            },
        });
    }
    Ok(output)
}

pub fn parse_dhcp(payload: &[u8]) -> anyhow::Result<PassiveObservationInput> {
    if payload.len() < 240
        || payload.len() > MAX_DHCP_BYTES
        || payload[236..240] != [99, 130, 83, 99]
    {
        bail!("DHCP payload is malformed or outside parser bounds");
    }
    let hlen = usize::from(payload[2]);
    if !matches!(payload[0], 1 | 2) || hlen == 0 || hlen > 16 {
        bail!("DHCP fixed header is invalid");
    }
    let options = dhcp_options(&payload[240..])?;
    let message_type = match options.get(&53).and_then(|value| value.first()).copied() {
        Some(1) => DhcpMessageType::Discover,
        Some(2) => DhcpMessageType::Offer,
        Some(3) => DhcpMessageType::Request,
        Some(4) => DhcpMessageType::Decline,
        Some(5) => DhcpMessageType::Ack,
        Some(6) => DhcpMessageType::Nak,
        Some(7) => DhcpMessageType::Release,
        Some(8) => DhcpMessageType::Inform,
        _ => bail!("DHCP message type is missing or unsupported"),
    };
    let server_message = matches!(
        message_type,
        DhcpMessageType::Offer | DhcpMessageType::Ack | DhcpMessageType::Nak
    );
    if (server_message && payload[0] != 2) || (!server_message && payload[0] != 1) {
        bail!("DHCP operation conflicts with message type");
    }
    let client_mac = if payload[1] == 1 && hlen == 6 {
        Some(MacAddress::new(payload[28..34].try_into().unwrap()))
    } else {
        None
    };
    let assigned_address = nonzero_ipv4(&payload[16..20]).or_else(|| {
        matches!(message_type, DhcpMessageType::Ack)
            .then(|| nonzero_ipv4(&payload[12..16]))
            .flatten()
    });
    let requested_address = option_ipv4(&options, 50)?;
    let server_address = option_ipv4(&options, 54)?.or_else(|| nonzero_ipv4(&payload[20..24]));
    let lease_seconds = option_u32(&options, 51)?;
    let observed_at = Utc::now();
    Ok(PassiveObservationInput {
        observation_id: Uuid::new_v4(),
        source: PassiveSource::Dhcp,
        confidence: 85,
        observed_at,
        expires_at: if matches!(message_type, DhcpMessageType::Offer | DhcpMessageType::Ack) {
            lease_seconds
                .map(|seconds| observed_at + Duration::seconds(i64::from(seconds.min(2_592_000))))
        } else {
            None
        },
        fact: PassiveFact::DhcpLease {
            message_type,
            transaction_id: hex::encode(&payload[4..8]),
            client_mac,
            assigned_address,
            requested_address,
            server_address,
            lease_seconds,
            hostname: option_text(&options, 12),
            vendor_class: option_text(&options, 60),
            routers: option_ipv4_list(&options, 3)?,
            dns_servers: option_ipv4_list(&options, 6)?,
            domain_name: option_text(&options, 15),
        },
    })
}

#[cfg(any(target_os = "linux", test))]
pub fn parse_proc_arp(contents: &str) -> Vec<PassiveObservationInput> {
    let observed_at = Utc::now();
    contents
        .lines()
        .skip(1)
        .take(4096)
        .filter_map(|line| {
            let columns: Vec<_> = line.split_whitespace().collect();
            if columns.len() < 6 {
                return None;
            }
            let address = IpAddr::V4(columns[0].parse().ok()?);
            let flags = u32::from_str_radix(columns[2].trim_start_matches("0x"), 16).ok()?;
            let mac_address = if flags & 0x2 != 0 {
                MacAddress::from_str(columns[3]).ok()
            } else {
                None
            };
            Some(PassiveObservationInput {
                observation_id: Uuid::new_v4(),
                source: PassiveSource::KernelNeighbor,
                confidence: if mac_address.is_some() { 80 } else { 35 },
                observed_at,
                expires_at: Some(observed_at + Duration::minutes(30)),
                fact: PassiveFact::NeighborMapping {
                    address,
                    mac_address,
                    interface: columns[5].chars().take(255).collect(),
                    state: if flags & 0x2 != 0 {
                        NeighborState::Reachable
                    } else {
                        NeighborState::Incomplete
                    },
                },
            })
        })
        .collect()
}

#[cfg(any(target_os = "linux", test))]
pub fn parse_ip_neighbor_json(contents: &[u8]) -> anyhow::Result<Vec<PassiveObservationInput>> {
    if contents.len() > 1024 * 1024 {
        bail!("kernel neighbor output exceeds parser bound");
    }
    let records = serde_json::from_slice::<serde_json::Value>(contents)?;
    let records = records
        .as_array()
        .ok_or_else(|| anyhow!("kernel neighbor output is not an array"))?;
    let observed_at = Utc::now();
    let mut output = Vec::new();
    for record in records.iter().take(4096) {
        let Some(interface) = record.get("dev").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let Some(address) = record
            .get("dst")
            .and_then(serde_json::Value::as_str)
            .and_then(|value| value.parse::<IpAddr>().ok())
        else {
            continue;
        };
        let mac_address = record
            .get("lladdr")
            .and_then(serde_json::Value::as_str)
            .and_then(|value| MacAddress::from_str(value).ok());
        let state_text = record
            .get("state")
            .and_then(|value| {
                value.as_str().or_else(|| {
                    value
                        .as_array()
                        .and_then(|items| items.first())
                        .and_then(serde_json::Value::as_str)
                })
            })
            .unwrap_or("unknown")
            .to_ascii_lowercase();
        let state = match state_text.as_str() {
            "permanent" | "noarp" => NeighborState::Permanent,
            "reachable" => NeighborState::Reachable,
            "stale" => NeighborState::Stale,
            "delay" => NeighborState::Delay,
            "probe" => NeighborState::Probe,
            "failed" => NeighborState::Failed,
            "incomplete" => NeighborState::Incomplete,
            _ => NeighborState::Unknown,
        };
        output.push(PassiveObservationInput {
            observation_id: Uuid::new_v4(),
            source: PassiveSource::KernelNeighbor,
            confidence: if mac_address.is_some() { 80 } else { 35 },
            observed_at,
            expires_at: Some(observed_at + Duration::minutes(30)),
            fact: PassiveFact::NeighborMapping {
                address,
                mac_address,
                interface: interface.chars().take(255).collect(),
                state,
            },
        });
    }
    Ok(output)
}

fn dns_name(packet: &[u8], start: usize) -> anyhow::Result<(String, usize)> {
    let mut labels = Vec::new();
    let mut cursor = start;
    let mut next = None;
    let mut jumps = 0;
    loop {
        let length = *packet
            .get(cursor)
            .ok_or_else(|| anyhow!("DNS name is truncated"))?;
        if length & 0xc0 == 0xc0 {
            let low = *packet
                .get(cursor + 1)
                .ok_or_else(|| anyhow!("DNS pointer is truncated"))?;
            let pointer = usize::from((u16::from(length & 0x3f) << 8) | u16::from(low));
            next.get_or_insert(cursor + 2);
            cursor = pointer;
            jumps += 1;
            if jumps > 16 {
                bail!("DNS compression pointer loop");
            }
            continue;
        }
        if length == 0 {
            let end = next.unwrap_or(cursor + 1);
            let name = labels.join(".");
            if name.len() > 255 {
                bail!("DNS name exceeds bound");
            }
            return Ok((name, end));
        }
        if length > 63 {
            bail!("DNS label exceeds bound");
        }
        let begin = cursor + 1;
        let end = begin + usize::from(length);
        let raw = packet
            .get(begin..end)
            .ok_or_else(|| anyhow!("DNS label is truncated"))?;
        let label = String::from_utf8_lossy(raw).to_string();
        if label.chars().any(char::is_control) {
            bail!("DNS label contains control characters");
        }
        labels.push(label);
        cursor = end;
    }
}

fn dhcp_options(payload: &[u8]) -> anyhow::Result<HashMap<u8, Vec<u8>>> {
    let mut result = HashMap::new();
    let mut offset = 0;
    let mut ended = false;
    while offset < payload.len() {
        let code = payload[offset];
        offset += 1;
        if code == 0 {
            continue;
        }
        if code == 255 {
            ended = true;
            break;
        }
        let length = usize::from(
            *payload
                .get(offset)
                .ok_or_else(|| anyhow!("DHCP option length is truncated"))?,
        );
        offset += 1;
        let end = offset + length;
        let value = payload
            .get(offset..end)
            .ok_or_else(|| anyhow!("DHCP option is truncated"))?;
        if result.insert(code, value.to_vec()).is_some() {
            bail!("duplicate DHCP option");
        }
        offset = end;
    }
    if !ended {
        bail!("DHCP options have no end marker");
    }
    Ok(result)
}

fn nonzero_ipv4(bytes: &[u8]) -> Option<Ipv4Addr> {
    let address = Ipv4Addr::new(bytes[0], bytes[1], bytes[2], bytes[3]);
    (!address.is_unspecified()).then_some(address)
}

fn option_ipv4(options: &HashMap<u8, Vec<u8>>, code: u8) -> anyhow::Result<Option<Ipv4Addr>> {
    match options.get(&code) {
        Some(value) if value.len() == 4 => Ok(nonzero_ipv4(value)),
        Some(_) => bail!("DHCP IPv4 option has invalid length"),
        None => Ok(None),
    }
}

fn option_u32(options: &HashMap<u8, Vec<u8>>, code: u8) -> anyhow::Result<Option<u32>> {
    match options.get(&code) {
        Some(value) if value.len() == 4 => Ok(Some(u32::from_be_bytes(
            value.as_slice().try_into().unwrap(),
        ))),
        Some(_) => bail!("DHCP integer option has invalid length"),
        None => Ok(None),
    }
}

fn option_ipv4_list(options: &HashMap<u8, Vec<u8>>, code: u8) -> anyhow::Result<Vec<Ipv4Addr>> {
    let Some(value) = options.get(&code) else {
        return Ok(vec![]);
    };
    if value.is_empty() || value.len() % 4 != 0 {
        bail!("DHCP address list is malformed");
    }
    Ok(value
        .chunks_exact(4)
        .take(8)
        .filter_map(nonzero_ipv4)
        .collect())
}

fn option_text(options: &HashMap<u8, Vec<u8>>, code: u8) -> Option<String> {
    let value = options.get(&code)?;
    let text = String::from_utf8_lossy(value)
        .trim_matches(char::from(0))
        .trim()
        .chars()
        .take(255)
        .collect::<String>();
    (!text.is_empty() && !text.chars().any(char::is_control)).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_name(output: &mut Vec<u8>, name: &str) {
        for label in name.split('.') {
            output.push(u8::try_from(label.len()).unwrap());
            output.extend_from_slice(label.as_bytes());
        }
        output.push(0);
    }

    fn push_record(output: &mut Vec<u8>, owner: &str, kind: u16, ttl: u32, data: &[u8]) {
        push_name(output, owner);
        output.extend_from_slice(&kind.to_be_bytes());
        output.extend_from_slice(&1u16.to_be_bytes());
        output.extend_from_slice(&ttl.to_be_bytes());
        output.extend_from_slice(&u16::try_from(data.len()).unwrap().to_be_bytes());
        output.extend_from_slice(data);
    }

    #[test]
    fn mdns_keeps_txt_keys_but_discards_values() {
        let service = "_ssh._tcp.local";
        let instance = "Printer._ssh._tcp.local";
        let host = "printer.local";
        let mut payload = vec![0, 0, 0x84, 0, 0, 0, 0, 4, 0, 0, 0, 0];
        let mut ptr = Vec::new();
        push_name(&mut ptr, instance);
        push_record(&mut payload, service, 12, 120, &ptr);
        let mut srv = vec![0, 0, 0, 0];
        srv.extend_from_slice(&22u16.to_be_bytes());
        push_name(&mut srv, host);
        push_record(&mut payload, instance, 33, 120, &srv);
        let txt_value = b"password=hunter2";
        let mut txt = vec![u8::try_from(txt_value.len()).unwrap()];
        txt.extend_from_slice(txt_value);
        push_record(&mut payload, instance, 16, 120, &txt);
        push_record(&mut payload, host, 1, 120, &[192, 0, 2, 10]);

        let observations = parse_mdns(&payload).unwrap();
        assert_eq!(observations.len(), 1);
        let json = serde_json::to_string(&observations).unwrap();
        assert!(json.contains("password"));
        assert!(!json.contains("hunter2"));
    }

    #[test]
    fn parses_bounded_dhcp_ack_without_retaining_payload() {
        let mut payload = vec![0u8; 240];
        payload[0] = 2;
        payload[1] = 1;
        payload[2] = 6;
        payload[4..8].copy_from_slice(&0x12345678u32.to_be_bytes());
        payload[16..20].copy_from_slice(&[192, 0, 2, 20]);
        payload[28..34].copy_from_slice(&[2, 0, 0, 0, 0, 1]);
        payload[236..240].copy_from_slice(&[99, 130, 83, 99]);
        payload.extend_from_slice(&[53, 1, 5, 51, 4, 0, 0, 0x0e, 0x10, 255]);
        let observation = parse_dhcp(&payload).unwrap();
        let json = serde_json::to_string(&observation).unwrap();
        assert!(json.contains("dhcp_lease"));
        assert!(!json.contains("63825363"));
    }

    #[test]
    fn proc_arp_normalization_is_bounded_and_structured() {
        let rows = "IP address HW type Flags HW address Mask Device\n192.0.2.2 0x1 0x2 02:00:00:00:00:01 * eth0\n";
        let observations = parse_proc_arp(rows);
        assert_eq!(observations.len(), 1);
        assert!(matches!(
            observations[0].fact,
            PassiveFact::NeighborMapping { .. }
        ));
    }

    #[test]
    fn ip_neighbor_normalization_preserves_state_without_raw_json() {
        let input = br#"[{"dst":"2001:db8::2","dev":"eth0","lladdr":"02:00:00:00:00:02","state":["STALE"],"unused":"not persisted"}]"#;
        let observations = parse_ip_neighbor_json(input).unwrap();
        assert_eq!(observations.len(), 1);
        let json = serde_json::to_string(&observations).unwrap();
        assert!(json.contains("stale"));
        assert!(!json.contains("unused"));
    }

    #[test]
    fn dns_pointer_loops_are_rejected() {
        assert!(dns_name(&[0xc0, 0x00], 0).is_err());
    }
}

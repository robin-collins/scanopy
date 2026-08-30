//! Opportunistic local LLDP collection through lldpd's supported CLI.
//!
//! This is a self-report source, not another network integration: it describes the daemon host's
//! own chassis and the peers seen on its NICs. `json0` is intentionally requested because lldpd
//! documents ordinary JSON as changing shape with the number of interfaces and neighbours. The
//! parser still accepts both shapes so older or differently-built clients remain usable.

use std::collections::BTreeMap;
use std::io;
use std::net::IpAddr;
use std::path::Path;
use std::process::Output;
use std::time::Duration;

use serde_json::{Map, Value};
use tokio::process::Command;
use tokio::time::timeout;

use crate::server::interfaces::r#impl::base::Interface;
use crate::server::lldp::{LldpChassisId, LldpPortId};

const LLDPCLI_TIMEOUT: Duration = Duration::from_secs(3);
const LLDPD_SOCKET_PATHS: [&str; 2] = ["/run/lldpd/lldpd.socket", "/var/run/lldpd.socket"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalLldpNeighbour {
    pub chassis_id: LldpChassisId,
    pub port_id: Option<LldpPortId>,
    pub sys_name: Option<String>,
    pub port_desc: Option<String>,
    pub mgmt_addr: Option<IpAddr>,
    pub sys_desc: Option<String>,
}

/// Everything the local lldpd source could establish in one collection.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LocalLldpSnapshot {
    pub chassis_id: Option<LldpChassisId>,
    pub neighbours: BTreeMap<String, LocalLldpNeighbour>,
    /// False when the neighbour command failed or several peers occupied one local interface.
    /// In that case the rows we did collect are useful, but absence must not clear stored data.
    pub neighbours_complete: bool,
}

impl LocalLldpSnapshot {
    /// Collect through `lldpcli` when it is installed and can reach lldpd.
    ///
    /// Absence is expected and silent: this source is opt-in by installing lldpd locally, or by
    /// mounting its socket into the daemon container whose image includes the client.
    pub async fn collect() -> Option<Self> {
        let socket = LLDPD_SOCKET_PATHS
            .iter()
            .copied()
            .find(|path| Path::new(path).exists());

        let chassis_output = match run_lldpcli(socket, &["show", "chassis", "details"]).await {
            Ok(output) => Some(output),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                tracing::debug!("lldpcli is not installed; skipping local LLDP self-report");
                return None;
            }
            Err(error) => {
                log_collection_failure(socket, "local chassis", &error);
                None
            }
        };

        let neighbour_output = match run_lldpcli(socket, &["show", "neighbors", "details"]).await {
            Ok(output) => Some(output),
            Err(error) => {
                log_collection_failure(socket, "local neighbours", &error);
                None
            }
        };

        if chassis_output.is_none() && neighbour_output.is_none() {
            return None;
        }

        let chassis_id = chassis_output.as_deref().and_then(|json| {
            match parse_local_chassis(json) {
                Ok(chassis) => chassis,
                Err(error) => {
                    tracing::warn!(error = %error, "Could not parse lldpcli local chassis JSON");
                    None
                }
            }
        });

        let (neighbours, neighbours_complete) = match neighbour_output.as_deref() {
            Some(json) => match parse_neighbours(json) {
                Ok(parsed) => parsed,
                Err(error) => {
                    tracing::warn!(error = %error, "Could not parse lldpcli neighbour JSON");
                    (BTreeMap::new(), false)
                }
            },
            None => (BTreeMap::new(), false),
        };

        Some(Self {
            chassis_id,
            neighbours,
            neighbours_complete,
        })
    }

    /// Attach each reported peer to the daemon host's existing NIC row by interface name.
    pub fn enrich_interfaces(&self, interfaces: &mut [Interface]) {
        for (name, neighbour) in &self.neighbours {
            let Some(interface) = interfaces.iter_mut().find(|interface| {
                interface.base.if_name.as_deref() == Some(name) || interface.base.if_descr == *name
            }) else {
                tracing::debug!(
                    interface = name,
                    "lldpd reported a filtered or unknown local NIC"
                );
                continue;
            };

            interface.base.lldp_chassis_id = Some(neighbour.chassis_id.clone());
            interface.base.lldp_port_id = neighbour.port_id.clone();
            interface.base.lldp_sys_name = neighbour.sys_name.clone();
            interface.base.lldp_port_desc = neighbour.port_desc.clone();
            interface.base.lldp_mgmt_addr = neighbour.mgmt_addr;
            interface.base.lldp_sys_desc = neighbour.sys_desc.clone();
        }
    }
}

async fn run_lldpcli(socket: Option<&str>, command: &[&str]) -> io::Result<Vec<u8>> {
    let binary = [
        "/usr/sbin/lldpcli",
        "/usr/bin/lldpcli",
        "/usr/local/sbin/lldpcli",
        "/usr/local/bin/lldpcli",
    ]
    .into_iter()
    .find(|path| Path::new(path).is_file())
    .unwrap_or("lldpcli");
    let mut process = Command::new(binary);
    process.args(["-f", "json0"]);
    if let Some(socket) = socket {
        process.args(["-u", socket]);
    }
    process.args(command);

    let output: Output = timeout(LLDPCLI_TIMEOUT, process.output())
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "lldpcli timed out"))??;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(io::Error::other(format!(
            "lldpcli exited with {}: {}",
            output.status,
            stderr.trim()
        )))
    }
}

fn log_collection_failure(socket: Option<&str>, subject: &str, error: &io::Error) {
    if socket.is_some() {
        tracing::warn!(?socket, error = %error, subject, "Local lldpd collection failed");
    } else {
        tracing::debug!(error = %error, subject, "Local lldpd is unavailable");
    }
}

fn parse_local_chassis(bytes: &[u8]) -> serde_json::Result<Option<LldpChassisId>> {
    let root: Value = serde_json::from_slice(bytes)?;
    Ok(values_for_key(&root, "chassis")
        .into_iter()
        .filter_map(|value| named_object_with_key(value, "id"))
        .map(|(object, _)| object)
        .find_map(parse_chassis))
}

/// Returns the usable neighbours and whether the JSON fit the one-peer-per-interface storage
/// model completely.
fn parse_neighbours(
    bytes: &[u8],
) -> serde_json::Result<(BTreeMap<String, LocalLldpNeighbour>, bool)> {
    let root: Value = serde_json::from_slice(bytes)?;
    let mut entries = Vec::new();
    let lldp = values_for_key(&root, "lldp");
    if lldp.is_empty() {
        return Ok((BTreeMap::new(), false));
    }
    for lldp in lldp {
        for interfaces in values_for_key(lldp, "interface") {
            expand_interface_entries(interfaces, None, &mut entries);
        }
    }

    let mut neighbours = BTreeMap::new();
    let mut complete = true;
    for (name_hint, entry) in entries {
        if direct_text(entry, "via").is_some_and(|via| !via.eq_ignore_ascii_case("lldp")) {
            continue;
        }
        let Some(name) = direct_text(entry, "name").or(name_hint) else {
            complete = false;
            continue;
        };
        let Some((chassis, chassis_name_hint)) = entry
            .get("chassis")
            .and_then(|value| named_object_with_key(value, "id"))
        else {
            complete = false;
            continue;
        };
        let Some(chassis_id) = parse_chassis(chassis) else {
            complete = false;
            continue;
        };
        let port = entry
            .get("port")
            .and_then(|value| named_object_with_key(value, "id"))
            .map(|(object, _)| object);
        let neighbour = LocalLldpNeighbour {
            chassis_id,
            port_id: port.and_then(parse_port),
            sys_name: direct_text(chassis, "name")
                .or(chassis_name_hint)
                .map(str::to_owned),
            port_desc: port
                .and_then(|value| direct_text(value, "descr"))
                .map(str::to_owned),
            mgmt_addr: first_parseable_ip(chassis, "mgmt-ip"),
            sys_desc: direct_text(chassis, "descr").map(str::to_owned),
        };

        if neighbours.insert(name.to_owned(), neighbour).is_some() {
            // The Interface model holds one peer. Keep the last deterministic CLI entry, but do
            // not claim the whole LLDP group was represented.
            complete = false;
        }
    }

    Ok((neighbours, complete))
}

fn parse_chassis(object: &Map<String, Value>) -> Option<LldpChassisId> {
    let (kind, value) = parse_id(object)?;
    let normalized = normalize_subtype(kind);
    match normalized.as_str() {
        "chassis" | "chassiscomponent" => Some(LldpChassisId::ChassisComponent(value.into())),
        "ifalias" | "interfacealias" => Some(LldpChassisId::InterfaceAlias(value.into())),
        "port" | "portcomponent" => Some(LldpChassisId::PortComponent(value.into())),
        "mac" | "macaddress" => match LldpChassisId::from_identifier_str(value) {
            id @ LldpChassisId::MacAddress(_) => Some(id),
            _ => None,
        },
        "ip" | "network" | "networkaddress" => {
            value.parse().ok().map(LldpChassisId::NetworkAddress)
        }
        "ifname" | "interfacename" => Some(LldpChassisId::InterfaceName(value.into())),
        "local" | "locallyassigned" => Some(LldpChassisId::LocallyAssigned(value.into())),
        _ => None,
    }
}

fn parse_port(object: &Map<String, Value>) -> Option<LldpPortId> {
    let (kind, value) = parse_id(object)?;
    let normalized = normalize_subtype(kind);
    match normalized.as_str() {
        "ifalias" | "interfacealias" => Some(LldpPortId::InterfaceAlias(value.into())),
        "port" | "portcomponent" => Some(LldpPortId::PortComponent(value.into())),
        "mac" | "macaddress" => match LldpChassisId::from_identifier_str(value) {
            LldpChassisId::MacAddress(mac) => Some(LldpPortId::MacAddress(mac)),
            _ => None,
        },
        "ip" | "network" | "networkaddress" => value.parse().ok().map(LldpPortId::NetworkAddress),
        "ifname" | "interfacename" => Some(LldpPortId::InterfaceName(value.into())),
        "agent" | "agentcircuitid" => Some(LldpPortId::AgentCircuitId(value.into())),
        "local" | "locallyassigned" => Some(LldpPortId::LocallyAssigned(value.into())),
        _ => None,
    }
}

fn parse_id(object: &Map<String, Value>) -> Option<(&str, &str)> {
    let id = object.get("id")?;
    let id = first_object(id)?;
    let kind = direct_text(id, "type")?.trim();
    let value = direct_text(id, "value")?.trim();
    (!kind.is_empty() && !value.is_empty()).then_some((kind, value))
}

fn normalize_subtype(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn values_for_key<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    fn visit<'a>(value: &'a Value, key: &str, found: &mut Vec<&'a Value>) {
        match value {
            Value::Object(object) => {
                for (name, child) in object {
                    if name == key {
                        found.push(child);
                    } else {
                        visit(child, key, found);
                    }
                }
            }
            Value::Array(values) => {
                for value in values {
                    visit(value, key, found);
                }
            }
            _ => {}
        }
    }

    let mut found = Vec::new();
    visit(value, key, &mut found);
    found
}

fn first_object(value: &Value) -> Option<&Map<String, Value>> {
    match value {
        Value::Object(object) => Some(object),
        Value::Array(values) => values.iter().find_map(first_object),
        _ => None,
    }
}

/// Finds an object containing `key`, retaining the immediately enclosing map key. lldpcli's
/// compact JSON writer uses a tag's `name` field as that enclosing key, whereas json0 retains the
/// name inside the object.
fn named_object_with_key<'a>(
    value: &'a Value,
    key: &str,
) -> Option<(&'a Map<String, Value>, Option<&'a str>)> {
    match value {
        Value::Object(object) if object.contains_key(key) => Some((object, None)),
        Value::Object(object) => object.iter().find_map(|(name, child)| {
            named_object_with_key(child, key)
                .map(|(found, hint)| (found, hint.or(Some(name.as_str()))))
        }),
        Value::Array(values) => values
            .iter()
            .find_map(|value| named_object_with_key(value, key)),
        _ => None,
    }
}

fn scalar_text(value: &Value) -> Option<&str> {
    match value {
        Value::String(text) => Some(text),
        Value::Array(values) => values.iter().find_map(scalar_text),
        Value::Object(object) => object.get("value").and_then(scalar_text),
        _ => None,
    }
}

fn direct_text<'a>(object: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    object.get(key).and_then(scalar_text)
}

fn first_parseable_ip(object: &Map<String, Value>, key: &str) -> Option<IpAddr> {
    fn visit(value: &Value) -> Option<IpAddr> {
        match value {
            Value::String(text) => text.parse().ok(),
            Value::Array(values) => values.iter().find_map(visit),
            Value::Object(object) => object.values().find_map(visit),
            _ => None,
        }
    }
    object.get(key).and_then(visit)
}

fn expand_interface_entries<'a>(
    value: &'a Value,
    name_hint: Option<&'a str>,
    entries: &mut Vec<(Option<&'a str>, &'a Map<String, Value>)>,
) {
    match value {
        Value::Array(values) => {
            for value in values {
                expand_interface_entries(value, name_hint, entries);
            }
        }
        Value::Object(object)
            if object.contains_key("chassis")
                || object.contains_key("port")
                || object.contains_key("via") =>
        {
            entries.push((name_hint, object));
        }
        Value::Object(object) => {
            for (name, value) in object {
                expand_interface_entries(value, Some(name), entries);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHASSIS_JSON0: &str = r#"{
      "local-chassis": [{"chassis": [{
        "id": [{"type": "mac", "value": "A0:B1:C2:D3:E4:F5"}],
        "name": [{"value": "scanopy-host"}]
      }]}]
    }"#;

    const NEIGHBOURS_JSON0: &str = r#"{
      "lldp": [{"interface": [
        {
          "name": "eno1", "via": "LLDP", "rid": "1",
          "chassis": [{
            "id": [{"type": "mac", "value": "00:11:22:33:44:55"}],
            "name": [{"value": "switch-core"}],
            "descr": [{"value": "Core switch"}],
            "mgmt-ip": [{"value": "192.0.2.10"}]
          }],
          "port": [{
            "id": [{"type": "ifname", "value": "Gi1/0/1"}],
            "descr": [{"value": "uplink"}]
          }]
        },
        {
          "name": "eno2", "via": "CDPv2",
          "chassis": [{"id": [{"type": "local", "value": "ignored"}]}],
          "port": [{"id": [{"type": "local", "value": "ignored"}]}]
        }
      ]}]
    }"#;

    #[test]
    fn parses_json0_chassis_into_the_shared_lldp_enum() {
        assert_eq!(
            parse_local_chassis(CHASSIS_JSON0.as_bytes()).unwrap(),
            Some(LldpChassisId::MacAddress("a0:b1:c2:d3:e4:f5".into()))
        );
    }

    #[test]
    fn parses_json0_neighbours_and_filters_other_protocols() {
        let (neighbours, complete) = parse_neighbours(NEIGHBOURS_JSON0.as_bytes()).unwrap();
        assert!(complete);
        assert_eq!(neighbours.len(), 1);
        assert_eq!(
            neighbours["eno1"],
            LocalLldpNeighbour {
                chassis_id: LldpChassisId::MacAddress("00:11:22:33:44:55".into()),
                port_id: Some(LldpPortId::InterfaceName("Gi1/0/1".into())),
                sys_name: Some("switch-core".into()),
                port_desc: Some("uplink".into()),
                mgmt_addr: Some("192.0.2.10".parse().unwrap()),
                sys_desc: Some("Core switch".into()),
            }
        );
    }

    #[test]
    fn parses_compact_json_keyed_by_interface_name() {
        let compact = br#"{
          "lldp": {"interface": {"eno9": {
            "via": "LLDP",
            "chassis": {"switch-compact": {
              "id": {"type": "local", "value": "server-rack"}
            }},
            "port": {"id": {"type": "mac", "value": "00-AB-24-89-CC-F0"}}
          }}}
        }"#;
        let (neighbours, complete) = parse_neighbours(compact).unwrap();
        assert!(complete);
        assert_eq!(
            neighbours["eno9"].chassis_id,
            LldpChassisId::LocallyAssigned("server-rack".into())
        );
        assert_eq!(
            neighbours["eno9"].port_id,
            Some(LldpPortId::MacAddress("00:ab:24:89:cc:f0".into()))
        );
        assert_eq!(
            neighbours["eno9"].sys_name.as_deref(),
            Some("switch-compact")
        );
    }

    #[test]
    fn parses_compact_chassis_name_key() {
        let compact = br#"{
          "local-chassis": {"chassis": {"scanopy-host": {
            "id": {"type": "ifname", "value": "eno1"}
          }}}
        }"#;
        assert_eq!(
            parse_local_chassis(compact).unwrap(),
            Some(LldpChassisId::InterfaceName("eno1".into()))
        );
    }

    #[test]
    fn duplicate_peers_are_retained_but_not_claimed_complete() {
        let duplicate =
            NEIGHBOURS_JSON0.replace("eno2\", \"via\": \"CDPv2", "eno1\", \"via\": \"LLDP");
        let (neighbours, complete) = parse_neighbours(duplicate.as_bytes()).unwrap();
        assert!(!complete);
        assert_eq!(neighbours.len(), 1);
    }

    #[test]
    fn valid_but_unrecognised_json_is_not_authoritative() {
        let (neighbours, complete) = parse_neighbours(br#"{"interfaces": []}"#).unwrap();
        assert!(neighbours.is_empty());
        assert!(!complete);
    }

    #[test]
    fn snapshot_enriches_the_matching_nic_only() {
        let (neighbours, complete) = parse_neighbours(NEIGHBOURS_JSON0.as_bytes()).unwrap();
        let snapshot = LocalLldpSnapshot {
            chassis_id: None,
            neighbours,
            neighbours_complete: complete,
        };
        let matching = Interface::new(crate::server::interfaces::r#impl::base::InterfaceBase {
            if_descr: "eno1".into(),
            if_name: Some("eno1".into()),
            ..Default::default()
        });
        let untouched = Interface::new(crate::server::interfaces::r#impl::base::InterfaceBase {
            if_descr: "eno8".into(),
            if_name: Some("eno8".into()),
            ..Default::default()
        });
        let mut interfaces = [matching.clone(), untouched.clone()];
        snapshot.enrich_interfaces(&mut interfaces);
        assert_eq!(
            interfaces[0].base.lldp_chassis_id,
            Some(LldpChassisId::MacAddress("00:11:22:33:44:55".into()))
        );
        assert_eq!(interfaces[1].base, untouched.base);
    }
}

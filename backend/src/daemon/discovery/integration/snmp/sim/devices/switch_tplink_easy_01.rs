use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::mibs::{BridgeTable, TplinkPrivateFdbEntry};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;

use super::inline;

/// TP-Link's entry-tier "Easy Smart" switches, unlike the Omada/JetStream L2+ models modelled by
/// `switch-omada-01` and `switch-tplink-01`, do not implement BRIDGE-MIB or Q-BRIDGE-MIB at
/// all — every column of both walks answers `noSuchObject`. Confirmed live on a TL-SG2218,
/// firmware `1.1.8 Build 20230602 Rel.73473` (see `TL-SG2218.md` at the repo root). What they do
/// serve is their own private FDB MIB (`tplinkl2BridgeMIB`, `oids::bridge::tplink_private`),
/// whose port column is a display string (`"1/0/6"`) resolved against `ifDescr` rather than a
/// `dot1dBasePort` number.
pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-tplink-easy-01",
        ip: Ipv4Addr::new(192, 168, 7, 253),
        purpose: Purpose::Regression {
            issue: "the TL-SG2218 field report, September 2026",
            defect: "no standard BRIDGE-MIB/Q-BRIDGE-MIB at all, so relying on those alone \
                     silently produced no MAC-table placement for non-LLDP devices behind it — \
                     even though the switch's own private FDB MIB has the same data",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("public"),
        },
        system: SystemInfo {
            sys_descr: Some("JetStream 16-Port Gigabit Smart Switch with 2 SFP Slots".into()),
            // TP-Link's enterprise OID (1.3.6.1.4.1.11863) is what the fallback keys on; the
            // exact suffix here is illustrative — the real switch's sysObjectID was not captured.
            sys_object_id: Some("1.3.6.1.4.1.11863.1.1.18".into()),
            sys_name: Some("TL-SG2218".into()),
            sys_location: Some("TeamCollins Homelab".into()),
            sys_contact: None,
            sys_services: Some(2),
            sys_uptime: None,
            if_number: None,
        },
        tables: tables(),
        arp_handler: Handler::Normal,
        suppresses: Vec::new(),
    }
}

fn tables() -> Tables {
    Tables {
        if_table: Some(if_table()),
        // No `lldp`, `bridge_ports` or standard bridge/Q-BRIDGE fdb — this device's whole point
        // is that those are absent. Only the private FDB table is served.
        bridge: BridgeTable::default().tplink_private_fdb(private_fdb()),
        ..Default::default()
    }
}

pub fn if_table() -> IfTable {
    IfTable::new(vec![
        IfRow::port(
            49155,
            "gigabitEthernet 1/0/3",
            Some("a0:36:9f:a8:89:ee".parse().unwrap()),
        ),
        IfRow::port(
            49157,
            "gigabitEthernet 1/0/5",
            Some("a0:36:9f:a8:89:ef".parse().unwrap()),
        ),
    ])
}

/// Three rows matching the live TL-SG2218 capture: two on ports this device's `ifTable` actually
/// has, one on a port it does not — a `dot1dTpFdbTable`-style pruned trunk/uplink port the FDB
/// still lists but the local `ifTable` walk truncated, or simply a port whose `ifDescr` uses a
/// naming convention (`ten-gigabitEthernet`, an SFP slot, ...) this fixture didn't bother
/// modelling. Either way, `if_index` must come back `None` for it rather than a wrong guess.
fn private_fdb() -> Vec<TplinkPrivateFdbEntry> {
    vec![
        TplinkPrivateFdbEntry {
            mac: "a0:36:9f:a8:89:ee".parse().unwrap(),
            vlan: 1,
            port: "1/0/3",
        },
        TplinkPrivateFdbEntry {
            mac: "a0:36:9f:a8:89:ef".parse().unwrap(),
            vlan: 1,
            port: "1/0/5",
        },
        TplinkPrivateFdbEntry {
            mac: "fc:34:97:a3:53:8b".parse().unwrap(),
            vlan: 1,
            port: "1/0/11",
        },
    ]
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::sim::harness;

    /// The point of this device: standard BRIDGE-MIB/Q-BRIDGE-MIB is fully absent, and the
    /// fallback still recovers the MAC table via TP-Link's private MIB.
    #[tokio::test]
    async fn a_device_with_no_standard_bridge_mib_still_yields_its_fdb() {
        let scan = harness::collect(&super::device()).await;

        assert!(
            scan.fdb.records.len() == 3,
            "all three private-FDB rows should surface, got {:?}",
            scan.fdb.records
        );
        assert!(
            !scan.fdb.unsupported,
            "the private-MIB fallback succeeded; this must not still read as unsupported"
        );
    }

    /// The two MACs on ports this device's `ifTable` actually has resolve to the right `if_index`
    /// — matched by the last whitespace-separated token of `ifDescr`, not a bridge-port number.
    #[tokio::test]
    async fn private_fdb_ports_resolve_against_if_descr() {
        let scan = harness::collect(&super::device()).await;

        let krusty = scan
            .fdb
            .records
            .iter()
            .find(|e| e.mac_address.to_string() == "A0:36:9F:A8:89:EE")
            .expect("the krusty-shaped MAC");
        assert_eq!(krusty.if_index, Some(49155));

        let ralph = scan
            .fdb
            .records
            .iter()
            .find(|e| e.mac_address.to_string() == "A0:36:9F:A8:89:EF")
            .expect("the ralph-shaped MAC");
        assert_eq!(ralph.if_index, Some(49157));
    }

    /// A row naming a port this device's `ifTable` doesn't have gets `if_index: None`, not a
    /// wrong guess — the row is still returned rather than silently dropped.
    #[tokio::test]
    async fn an_unresolvable_port_string_yields_no_if_index_but_keeps_the_row() {
        let scan = harness::collect(&super::device()).await;

        let unresolved = scan
            .fdb
            .records
            .iter()
            .find(|e| e.mac_address.to_string() == "FC:34:97:A3:53:8B")
            .expect("the row for the port this fixture never modelled");
        assert_eq!(unresolved.if_index, None);
    }
}

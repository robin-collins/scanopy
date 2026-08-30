use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, LocalPort, RemoteNeighbour, V2,
};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::wire::MacEncoding;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::lldp::{LldpChassisId, LldpPortId};

use super::inline;

pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-ufispace-01",
        ip: Ipv4Addr::new(192, 168, 7, 252),
        purpose: Purpose::Regression {
            issue: "#688",
            defect: "OcNOS serves neighbours only through LLDP-V2-MIB, whose remote index already contains IF-MIB ifIndex",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("UfiSpace S9500-30XS, OcNOS".into()),
            sys_object_id: Some("1.3.6.1.4.1.50114.1".into()),
            sys_name: Some("switch-ufispace-01".into()),
            sys_location: Some("Spine rack".into()),
            sys_contact: Some("netops@example.com".into()),
            sys_services: Some(6),
            sys_uptime: None,
            if_number: None,
        },
        tables: Tables {
            if_table: Some(if_table()),
            lldp: Some(lldp_table()),
            ..Default::default()
        },
        arp_handler: Handler::Normal,
        suppresses: Vec::new(),
    }
}

pub fn if_table() -> IfTable {
    IfTable::new(vec![
        IfRow::port(10009, "xe1", Some("00:11:22:33:44:09".parse().unwrap()))
            .speed(10000000000)
            .name("xe1"),
        IfRow::port(10073, "xe2", Some("00:11:22:33:44:49".parse().unwrap()))
            .speed(10000000000)
            .name("xe2"),
    ])
}

pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::octets(LldpChassisId::MacAddress("00:11:22:33:44:00".into())),
        "switch-ufispace-01",
    )
    .in_mib(&V2)
    .sys_desc("UfiSpace S9500-30XS, OcNOS")
    .local_ports(vec![
        LocalPort::new(
            10009,
            Advertised::text(
                LldpPortId::InterfaceName("xe1".into()),
                MacEncoding::AsciiLower,
            ),
        ),
        LocalPort::new(
            10073,
            Advertised::text(
                LldpPortId::InterfaceName("xe2".into()),
                MacEncoding::AsciiLower,
            ),
        ),
    ])
    .neighbours(vec![
        RemoteNeighbour::new(
            10009,
            Advertised::octets(LldpChassisId::MacAddress("00:1a:2b:00:10:00".into())),
            Advertised::text(
                LldpPortId::InterfaceName("Gi0/1".into()),
                MacEncoding::AsciiLower,
            ),
        )
        .index(6)
        .sys_name("switch-core-01"),
        RemoteNeighbour::new(
            10073,
            Advertised::octets(LldpChassisId::MacAddress("00:1a:2b:00:11:00".into())),
            Advertised::text(
                LldpPortId::InterfaceName("Gi0/2".into()),
                MacEncoding::AsciiLower,
            ),
        )
        .index(2)
        .sys_name("switch-access-01"),
    ])
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::queries::query_lldp_local;
    use crate::daemon::discovery::integration::snmp::sim::harness;
    use crate::server::lldp::LldpChassisId;

    #[tokio::test]
    async fn v2_only_neighbours_keep_their_real_if_indexes() {
        let scan = harness::scan("switch-ufispace-01").await;

        assert_eq!(scan.neighbours.records.len(), 2);
        assert_eq!(
            scan.neighbour_named("switch-core-01").local_port_index,
            10009
        );
        assert_eq!(
            scan.neighbour_named("switch-access-01").local_port_index,
            10073
        );
        assert_eq!(scan.local_port_outcome.unmatched, 0);
        assert_eq!(scan.dropped_neighbours, 0);
    }

    #[tokio::test]
    async fn v2_only_local_identity_is_collected() {
        let device = super::device();
        let mut agent = device.agent();
        let local = query_lldp_local(&mut agent, device.ip.into())
            .await
            .unwrap()
            .expect("V2 local chassis identity");

        assert_eq!(
            LldpChassisId::from_snmp(local.chassis_id_subtype, &local.chassis_id_bytes),
            Some(LldpChassisId::MacAddress("00:11:22:33:44:00".into()))
        );
    }
}

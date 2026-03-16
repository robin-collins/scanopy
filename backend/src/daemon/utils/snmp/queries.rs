//! SNMP Query Functions
//!
//! Functions for querying SNMP data from devices.

use anyhow::{Result, anyhow};
use snmp2::{Oid, Value};
use std::collections::HashMap;
use std::net::IpAddr;
use tokio::time::timeout;
use tracing::{debug, trace, warn};

use crate::server::credentials::r#impl::mapping::SnmpQueryCredential;

use super::oids::{self, oid_to_vec, parse_oid};
use super::session::{MAX_WALK_ENTRIES, SNMP_TIMEOUT, create_session};
use super::types::{CdpNeighbor, IfTableEntry, LldpNeighbor, SystemInfo};
use super::values::{
    parse_lldp_mgmt_addr, value_to_i32, value_to_mac, value_to_string, value_to_u64,
};

/// Query system MIB information from a device
pub async fn query_system_info(ip: IpAddr, credential: &SnmpQueryCredential) -> Result<SystemInfo> {
    let mut session = create_session(ip, credential).await?;
    let mut info = SystemInfo::default();

    // Query each system OID
    let oids_to_query = [
        (oids::system::SYS_DESCR, "sysDescr"),
        (oids::system::SYS_OBJECT_ID, "sysObjectID"),
        (oids::system::SYS_NAME, "sysName"),
        (oids::system::SYS_LOCATION, "sysLocation"),
        (oids::system::SYS_CONTACT, "sysContact"),
        (oids::system::SYS_UPTIME, "sysUpTime"),
    ];

    for (oid_str, name) in oids_to_query {
        let oid = match parse_oid(oid_str) {
            Ok(o) => o,
            Err(e) => {
                warn!("Failed to parse OID {}: {}", oid_str, e);
                continue;
            }
        };

        match timeout(SNMP_TIMEOUT, session.get(&oid)).await {
            Ok(Ok(mut response)) => {
                if let Some((resp_oid, value)) = response.varbinds.next() {
                    trace!("SNMP {} from {}: {:?} = {:?}", name, ip, resp_oid, value);
                    match name {
                        "sysDescr" => info.sys_descr = value_to_string(&value),
                        "sysObjectID" => info.sys_object_id = value_to_string(&value),
                        "sysName" => info.sys_name = value_to_string(&value),
                        "sysLocation" => info.sys_location = value_to_string(&value),
                        "sysContact" => info.sys_contact = value_to_string(&value),
                        "sysUpTime" => info.sys_uptime = value_to_u64(&value),
                        _ => {}
                    }
                }
            }
            Ok(Err(e)) => {
                debug!("SNMP GET {} failed from {}: {:?}", name, ip, e);
            }
            Err(_) => {
                debug!("SNMP GET {} timeout from {}", name, ip);
            }
        }
    }

    Ok(info)
}

/// Walk the ifTable and ifXTable to get interface information
pub async fn walk_if_table(
    ip: IpAddr,
    credential: &SnmpQueryCredential,
) -> Result<Vec<IfTableEntry>> {
    let mut session = create_session(ip, credential).await?;
    let mut entries: HashMap<i32, IfTableEntry> = HashMap::new();

    // Define the columns we want to walk
    let columns = [
        (oids::if_mib::columns::IF_INDEX, "ifIndex"),
        (oids::if_mib::columns::IF_DESCR, "ifDescr"),
        (oids::if_mib::columns::IF_TYPE, "ifType"),
        (oids::if_mib::columns::IF_MTU, "ifMtu"),
        (oids::if_mib::columns::IF_SPEED, "ifSpeed"),
        (oids::if_mib::columns::IF_PHYS_ADDRESS, "ifPhysAddress"),
        (oids::if_mib::columns::IF_ADMIN_STATUS, "ifAdminStatus"),
        (oids::if_mib::columns::IF_OPER_STATUS, "ifOperStatus"),
        (oids::if_mib::if_x_table::IF_NAME, "ifName"),
        (oids::if_mib::if_x_table::IF_HIGH_SPEED, "ifHighSpeed"),
        (oids::if_mib::if_x_table::IF_ALIAS, "ifAlias"),
    ];

    // Walk each column
    for (base_oid_str, column_name) in columns {
        let base_oid = match parse_oid(base_oid_str) {
            Ok(o) => o,
            Err(e) => {
                warn!("Failed to parse OID {}: {}", base_oid_str, e);
                continue;
            }
        };

        let base_parts: Vec<u64> = base_oid_str
            .split('.')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();

        let mut current_oid = base_oid.clone();
        let mut count = 0;

        loop {
            if count >= MAX_WALK_ENTRIES {
                warn!("Walk limit reached for {} on {}", column_name, ip);
                break;
            }

            match timeout(SNMP_TIMEOUT, session.getnext(&current_oid)).await {
                Ok(Ok(mut response)) => {
                    if let Some((resp_oid, value)) = response.varbinds.next() {
                        // Check if we're still in the same subtree
                        let response_parts = oid_to_vec(&resp_oid);
                        if response_parts.len() <= base_parts.len()
                            || !response_parts.starts_with(&base_parts)
                        {
                            // We've walked past the column
                            break;
                        }

                        // Extract ifIndex from OID (last component)
                        if let Some(&if_index_u64) = response_parts.last() {
                            let if_index = if_index_u64 as i32;
                            let entry = entries.entry(if_index).or_insert_with(|| IfTableEntry {
                                if_index,
                                if_descr: None,
                                if_type: None,
                                if_mtu: None,
                                if_speed: None,
                                if_phys_address: None,
                                if_admin_status: None,
                                if_oper_status: None,
                                if_name: None,
                                if_alias: None,
                            });

                            match column_name {
                                "ifIndex" => {
                                    // Already set above
                                }
                                "ifDescr" => entry.if_descr = value_to_string(&value),
                                "ifType" => entry.if_type = value_to_i32(&value),
                                "ifMtu" => entry.if_mtu = value_to_i32(&value),
                                "ifSpeed" => {
                                    // Only set if ifHighSpeed not already set
                                    if entry.if_speed.is_none() {
                                        entry.if_speed = value_to_u64(&value);
                                    }
                                }
                                "ifPhysAddress" => entry.if_phys_address = value_to_mac(&value),
                                "ifAdminStatus" => entry.if_admin_status = value_to_i32(&value),
                                "ifOperStatus" => entry.if_oper_status = value_to_i32(&value),
                                "ifName" => entry.if_name = value_to_string(&value),
                                "ifHighSpeed" => {
                                    // ifHighSpeed is in Mbps, convert to bps for consistency
                                    if let Some(mbps) = value_to_u64(&value) {
                                        entry.if_speed = Some(mbps * 1_000_000);
                                    }
                                }
                                "ifAlias" => entry.if_alias = value_to_string(&value),
                                _ => {}
                            }
                        }

                        current_oid = Oid::from(response_parts.as_slice())
                            .map_err(|e| anyhow!("Invalid response OID: {:?}", e))?;
                        count += 1;
                    } else {
                        break;
                    }
                }
                Ok(Err(e)) => {
                    debug!("Walk {} failed on {}: {:?}", column_name, ip, e);
                    break;
                }
                Err(_) => {
                    debug!("Walk {} timeout on {}", column_name, ip);
                    break;
                }
            }
        }

        trace!("Walked {} entries for {} from {}", count, column_name, ip);
    }

    let mut result: Vec<IfTableEntry> = entries.into_values().collect();
    result.sort_by_key(|e| e.if_index);

    debug!(
        "SNMP ifTable walk from {} returned {} interfaces",
        ip,
        result.len()
    );

    Ok(result)
}

/// Query LLDP remote table for neighbor information
pub async fn query_lldp_neighbors(
    ip: IpAddr,
    credential: &SnmpQueryCredential,
) -> Result<Vec<LldpNeighbor>> {
    let mut session = create_session(ip, credential).await?;
    let mut neighbors: HashMap<(i32, i32), LldpNeighbor> = HashMap::new();

    // LLDP remote table uses a complex index: lldpRemTimeMark.lldpRemLocalPortNum.lldpRemIndex
    // We'll walk the columns and extract the local port from the OID

    let columns = [
        (
            oids::lldp::remote::entry::LLDP_REM_CHASSIS_ID_SUBTYPE,
            "remChassisIdSubtype",
        ),
        (
            oids::lldp::remote::entry::LLDP_REM_CHASSIS_ID,
            "remChassisId",
        ),
        (
            oids::lldp::remote::entry::LLDP_REM_PORT_ID_SUBTYPE,
            "remPortIdSubtype",
        ),
        (oids::lldp::remote::entry::LLDP_REM_PORT_ID, "remPortId"),
        (oids::lldp::remote::entry::LLDP_REM_PORT_DESC, "remPortDesc"),
        (oids::lldp::remote::entry::LLDP_REM_SYS_NAME, "remSysName"),
        (oids::lldp::remote::entry::LLDP_REM_SYS_DESC, "remSysDesc"),
        (oids::lldp::remote::entry::LLDP_REM_MAN_ADDR, "remManAddr"),
    ];

    for (base_oid_str, column_name) in columns {
        let base_oid = match parse_oid(base_oid_str) {
            Ok(o) => o,
            Err(e) => {
                debug!("Failed to parse LLDP OID {}: {}", base_oid_str, e);
                continue;
            }
        };

        let base_parts: Vec<u64> = base_oid_str
            .split('.')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();

        let mut current_oid = base_oid.clone();
        let mut count = 0;

        loop {
            if count >= MAX_WALK_ENTRIES {
                break;
            }

            match timeout(SNMP_TIMEOUT, session.getnext(&current_oid)).await {
                Ok(Ok(mut response)) => {
                    if let Some((resp_oid, value)) = response.varbinds.next() {
                        let response_parts = oid_to_vec(&resp_oid);
                        if response_parts.len() <= base_parts.len()
                            || !response_parts.starts_with(&base_parts)
                        {
                            break;
                        }

                        // Extract index components from OID suffix
                        // Format: base.timeMark.localPortNum.remIndex
                        let suffix = &response_parts[base_parts.len()..];
                        if suffix.len() >= 3 {
                            let local_port = suffix[1] as i32;
                            let rem_index = suffix[2] as i32;

                            let neighbor =
                                neighbors.entry((local_port, rem_index)).or_insert_with(|| {
                                    LldpNeighbor {
                                        local_port_index: local_port,
                                        remote_chassis_id_subtype: None,
                                        remote_chassis_id_bytes: None,
                                        remote_port_id_subtype: None,
                                        remote_port_id_bytes: None,
                                        remote_port_desc: None,
                                        remote_sys_name: None,
                                        remote_sys_desc: None,
                                        remote_mgmt_addr: None,
                                    }
                                });

                            match column_name {
                                "remChassisIdSubtype" => {
                                    neighbor.remote_chassis_id_subtype =
                                        value_to_i32(&value).map(|v| v as u8)
                                }
                                "remChassisId" => {
                                    if let Value::OctetString(bytes) = &value {
                                        neighbor.remote_chassis_id_bytes = Some(bytes.to_vec());
                                    }
                                }
                                "remPortIdSubtype" => {
                                    neighbor.remote_port_id_subtype =
                                        value_to_i32(&value).map(|v| v as u8)
                                }
                                "remPortId" => {
                                    if let Value::OctetString(bytes) = &value {
                                        neighbor.remote_port_id_bytes = Some(bytes.to_vec());
                                    }
                                }
                                "remPortDesc" => {
                                    neighbor.remote_port_desc = value_to_string(&value)
                                }
                                "remSysName" => neighbor.remote_sys_name = value_to_string(&value),
                                "remSysDesc" => neighbor.remote_sys_desc = value_to_string(&value),
                                "remManAddr" => {
                                    // Management address is encoded as address family + address bytes
                                    if let Value::OctetString(bytes) = &value {
                                        neighbor.remote_mgmt_addr = parse_lldp_mgmt_addr(bytes);
                                    }
                                }
                                _ => {}
                            }
                        }

                        current_oid = Oid::from(response_parts.as_slice())
                            .map_err(|e| anyhow!("Invalid response OID: {:?}", e))?;
                        count += 1;
                    } else {
                        break;
                    }
                }
                Ok(Err(_)) | Err(_) => break,
            }
        }
    }

    let result: Vec<LldpNeighbor> = neighbors.into_values().collect();
    debug!("LLDP query from {} returned {} neighbors", ip, result.len());

    Ok(result)
}

/// Query ipAddrTable for IP address to ifIndex mappings.
/// Walks ipAdEntIfIndex (OID 1.3.6.1.2.1.4.20.1.2) where the OID suffix
/// encodes the IP address as A.B.C.D and the value is the ifIndex.
pub async fn query_ip_addr_table(
    ip: IpAddr,
    credential: &SnmpQueryCredential,
) -> Result<HashMap<IpAddr, i32>> {
    let mut session = create_session(ip, credential).await?;
    let mut result: HashMap<IpAddr, i32> = HashMap::new();

    let base_oid_str = oids::ip_mib::ip_addr_entry::IP_AD_ENT_IF_INDEX;
    let base_oid = parse_oid(base_oid_str)?;
    let base_parts: Vec<u64> = base_oid_str
        .split('.')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();

    let mut current_oid = base_oid.clone();
    let mut count = 0;

    loop {
        if count >= MAX_WALK_ENTRIES {
            warn!("Walk limit reached for ipAddrTable on {}", ip);
            break;
        }

        match timeout(SNMP_TIMEOUT, session.getnext(&current_oid)).await {
            Ok(Ok(mut response)) => {
                if let Some((resp_oid, value)) = response.varbinds.next() {
                    let response_parts = oid_to_vec(&resp_oid);
                    if response_parts.len() <= base_parts.len()
                        || !response_parts.starts_with(&base_parts)
                    {
                        break;
                    }

                    // OID suffix is the IP address: base.A.B.C.D
                    let suffix = &response_parts[base_parts.len()..];
                    if suffix.len() == 4 {
                        let addr = IpAddr::from([
                            suffix[0] as u8,
                            suffix[1] as u8,
                            suffix[2] as u8,
                            suffix[3] as u8,
                        ]);
                        if let Some(if_index) = value_to_i32(&value) {
                            result.insert(addr, if_index);
                        }
                    }

                    current_oid = Oid::from(response_parts.as_slice())
                        .map_err(|e| anyhow!("Invalid response OID: {:?}", e))?;
                    count += 1;
                } else {
                    break;
                }
            }
            Ok(Err(e)) => {
                debug!("ipAddrTable walk failed on {}: {:?}", ip, e);
                break;
            }
            Err(_) => {
                debug!("ipAddrTable walk timeout on {}", ip);
                break;
            }
        }
    }

    debug!(
        "ipAddrTable walk from {} returned {} entries",
        ip,
        result.len()
    );

    Ok(result)
}

/// Query CDP cache table for neighbor information (Cisco devices)
pub async fn query_cdp_neighbors(
    ip: IpAddr,
    credential: &SnmpQueryCredential,
) -> Result<Vec<CdpNeighbor>> {
    let mut session = create_session(ip, credential).await?;
    let mut neighbors: HashMap<(i32, i32), CdpNeighbor> = HashMap::new();

    let columns = [
        (oids::cdp::entry::CDP_CACHE_DEVICE_ID, "deviceId"),
        (oids::cdp::entry::CDP_CACHE_DEVICE_PORT, "devicePort"),
        (oids::cdp::entry::CDP_CACHE_PLATFORM, "platform"),
        (oids::cdp::entry::CDP_CACHE_ADDRESS, "address"),
    ];

    for (base_oid_str, column_name) in columns {
        let base_oid = match parse_oid(base_oid_str) {
            Ok(o) => o,
            Err(e) => {
                debug!("Failed to parse CDP OID {}: {}", base_oid_str, e);
                continue;
            }
        };

        let base_parts: Vec<u64> = base_oid_str
            .split('.')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();

        let mut current_oid = base_oid.clone();
        let mut count = 0;

        loop {
            if count >= MAX_WALK_ENTRIES {
                break;
            }

            match timeout(SNMP_TIMEOUT, session.getnext(&current_oid)).await {
                Ok(Ok(mut response)) => {
                    if let Some((resp_oid, value)) = response.varbinds.next() {
                        let response_parts = oid_to_vec(&resp_oid);
                        if response_parts.len() <= base_parts.len()
                            || !response_parts.starts_with(&base_parts)
                        {
                            break;
                        }

                        // CDP index: base.cdpCacheIfIndex.cdpCacheDeviceIndex
                        let suffix = &response_parts[base_parts.len()..];
                        if suffix.len() >= 2 {
                            let if_index = suffix[0] as i32;
                            let device_index = suffix[1] as i32;

                            let neighbor = neighbors
                                .entry((if_index, device_index))
                                .or_insert_with(|| CdpNeighbor {
                                    local_port_index: if_index,
                                    remote_device_id: None,
                                    remote_port_id: None,
                                    remote_platform: None,
                                    remote_address: None,
                                });

                            match column_name {
                                "deviceId" => neighbor.remote_device_id = value_to_string(&value),
                                "devicePort" => neighbor.remote_port_id = value_to_string(&value),
                                "platform" => neighbor.remote_platform = value_to_string(&value),
                                "address" => {
                                    // CDP address is encoded as 4 bytes for IPv4
                                    if let Value::OctetString(bytes) = &value
                                        && bytes.len() == 4
                                    {
                                        neighbor.remote_address = Some(IpAddr::from([
                                            bytes[0], bytes[1], bytes[2], bytes[3],
                                        ]));
                                    }
                                }
                                _ => {}
                            }
                        }

                        current_oid = Oid::from(response_parts.as_slice())
                            .map_err(|e| anyhow!("Invalid response OID: {:?}", e))?;
                        count += 1;
                    } else {
                        break;
                    }
                }
                Ok(Err(_)) | Err(_) => break,
            }
        }
    }

    let result: Vec<CdpNeighbor> = neighbors.into_values().collect();
    debug!("CDP query from {} returned {} neighbors", ip, result.len());

    Ok(result)
}

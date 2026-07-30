use std::{
    fmt,
    net::{IpAddr, Ipv4Addr},
};

use chrono::{DateTime, Utc};
use mac_address::MacAddress;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

pub const MAX_OBSERVATIONS_PER_BATCH: usize = 256;
pub const MAX_TEXT_BYTES: usize = 255;
pub const MAX_TXT_KEYS: usize = 32;
pub const MAX_DNS_SERVERS: usize = 8;
pub const MAX_REQUEST_BODY_BYTES: usize = 1024 * 1024;
pub const DEFAULT_RETENTION_DAYS: i64 = 30;
/// Hard storage ceiling per daemon/network, independent of time-based retention.
pub const MAX_STORED_OBSERVATIONS_PER_DAEMON: i64 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PassiveSource {
    Mdns,
    Dhcp,
    KernelNeighbor,
    Arp,
}

impl fmt::Display for PassiveSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Mdns => "mdns",
            Self::Dhcp => "dhcp",
            Self::KernelNeighbor => "kernel_neighbor",
            Self::Arp => "arp",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DhcpMessageType {
    Discover,
    Offer,
    Request,
    Decline,
    Ack,
    Nak,
    Release,
    Inform,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NeighborState {
    Permanent,
    Reachable,
    Stale,
    Delay,
    Probe,
    Failed,
    Incomplete,
    Unknown,
}

/// A bounded structured fact. There is deliberately no raw-payload or generic
/// JSON variant: adding a field requires review at both ends of the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PassiveFact {
    MdnsService {
        service_type: String,
        instance: String,
        hostname: Option<String>,
        #[schema(value_type = Vec<String>)]
        addresses: Vec<IpAddr>,
        port: Option<u16>,
        /// TXT keys only. Values can contain customer data and are discarded.
        txt_keys: Vec<String>,
        ttl_seconds: u32,
    },
    DhcpLease {
        message_type: DhcpMessageType,
        transaction_id: String,
        #[schema(value_type = Option<String>)]
        client_mac: Option<MacAddress>,
        #[schema(value_type = Option<String>)]
        assigned_address: Option<Ipv4Addr>,
        #[schema(value_type = Option<String>)]
        requested_address: Option<Ipv4Addr>,
        #[schema(value_type = Option<String>)]
        server_address: Option<Ipv4Addr>,
        lease_seconds: Option<u32>,
        hostname: Option<String>,
        vendor_class: Option<String>,
        #[schema(value_type = Vec<String>)]
        routers: Vec<Ipv4Addr>,
        #[schema(value_type = Vec<String>)]
        dns_servers: Vec<Ipv4Addr>,
        domain_name: Option<String>,
    },
    NeighborMapping {
        #[schema(value_type = String)]
        address: IpAddr,
        #[schema(value_type = Option<String>)]
        mac_address: Option<MacAddress>,
        interface: String,
        state: NeighborState,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PassiveObservationInput {
    pub observation_id: Uuid,
    pub source: PassiveSource,
    /// Integer percent avoids NaN/rounding ambiguity on the wire.
    pub confidence: u8,
    pub observed_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub fact: PassiveFact,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PassiveIngestRequest {
    pub network_id: Uuid,
    pub observations: Vec<PassiveObservationInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PassiveIngestResponse {
    pub accepted: u32,
    pub duplicates: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
pub struct PassiveObservation {
    pub id: Uuid,
    pub network_id: Uuid,
    pub daemon_id: Uuid,
    pub source: String,
    #[schema(value_type = i32)]
    pub confidence: i64,
    pub correlation_kind: String,
    pub correlation_key: String,
    #[sqlx(json)]
    pub fact: PassiveFact,
    pub observed_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct PassiveListQuery {
    pub network_id: Option<Uuid>,
    pub source: Option<String>,
    #[param(minimum = 1, maximum = 500)]
    pub limit: Option<u32>,
    #[param(minimum = 0)]
    pub offset: Option<u32>,
}

impl PassiveListQuery {
    pub fn pagination(&self) -> (u32, u32) {
        (
            self.limit.unwrap_or(100).clamp(1, 500),
            self.offset.unwrap_or(0),
        )
    }
}

impl PassiveIngestRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.observations.is_empty() || self.observations.len() > MAX_OBSERVATIONS_PER_BATCH {
            return Err(format!(
                "passive batches must contain 1 to {MAX_OBSERVATIONS_PER_BATCH} observations"
            ));
        }
        for observation in &self.observations {
            observation.validate()?;
        }
        Ok(())
    }
}

impl PassiveObservationInput {
    pub fn validate(&self) -> Result<(), String> {
        if self.confidence > 100 {
            return Err("passive confidence must be between 0 and 100".into());
        }
        let (expected_confidence, expected_expiry) = match (&self.source, &self.fact) {
            (PassiveSource::Mdns, PassiveFact::MdnsService { ttl_seconds, .. }) => (
                75,
                Some(self.observed_at + chrono::Duration::seconds(i64::from(*ttl_seconds))),
            ),
            (
                PassiveSource::Dhcp,
                PassiveFact::DhcpLease {
                    message_type,
                    lease_seconds,
                    ..
                },
            ) => (
                85,
                if matches!(message_type, DhcpMessageType::Offer | DhcpMessageType::Ack) {
                    lease_seconds.map(|seconds| {
                        self.observed_at
                            + chrono::Duration::seconds(i64::from(seconds.min(2_592_000)))
                    })
                } else {
                    None
                },
            ),
            (
                PassiveSource::KernelNeighbor,
                PassiveFact::NeighborMapping {
                    mac_address: Some(_),
                    ..
                },
            ) => (80, Some(self.observed_at + chrono::Duration::minutes(30))),
            (
                PassiveSource::KernelNeighbor,
                PassiveFact::NeighborMapping {
                    mac_address: None, ..
                },
            ) => (35, Some(self.observed_at + chrono::Duration::minutes(30))),
            (PassiveSource::Arp, PassiveFact::NeighborMapping { .. }) => {
                (70, Some(self.observed_at + chrono::Duration::minutes(30)))
            }
            _ => return Err("passive source does not match the structured fact type".into()),
        };
        if self.confidence != expected_confidence {
            return Err(
                "passive confidence does not match its server-defined source policy".into(),
            );
        }
        if self.expires_at != expected_expiry {
            return Err("passive expiry does not match its server-defined source policy".into());
        }
        if self.observed_at > Utc::now() + chrono::Duration::minutes(5) {
            return Err("passive observation timestamp is too far in the future".into());
        }
        if let Some(expires_at) = self.expires_at
            && (expires_at < self.observed_at
                || expires_at > self.observed_at + chrono::Duration::days(DEFAULT_RETENTION_DAYS))
        {
            return Err("passive observation expiry is outside the retention window".into());
        }
        self.fact.validate()
    }
}

impl PassiveFact {
    fn validate(&self) -> Result<(), String> {
        match self {
            Self::MdnsService {
                service_type,
                instance,
                hostname,
                addresses,
                txt_keys,
                ttl_seconds,
                ..
            } => {
                bounded_text(service_type, "mDNS service type")?;
                bounded_text(instance, "mDNS instance")?;
                optional_text(hostname, "mDNS hostname")?;
                if addresses.len() > 16 || txt_keys.len() > MAX_TXT_KEYS || *ttl_seconds > 86_400 {
                    return Err("mDNS observation exceeds a collection bound".into());
                }
                if addresses.iter().any(|address| !valid_ip(*address)) {
                    return Err("mDNS observation contains an unusable address".into());
                }
                for key in txt_keys {
                    bounded_text(key, "mDNS TXT key")?;
                }
            }
            Self::DhcpLease {
                transaction_id,
                hostname,
                vendor_class,
                routers,
                dns_servers,
                domain_name,
                client_mac,
                assigned_address,
                requested_address,
                server_address,
                ..
            } => {
                if transaction_id.len() != 8
                    || !transaction_id.bytes().all(|byte| byte.is_ascii_hexdigit())
                {
                    return Err("DHCP transaction ID must be eight hexadecimal characters".into());
                }
                optional_text(hostname, "DHCP hostname")?;
                optional_text(vendor_class, "DHCP vendor class")?;
                optional_text(domain_name, "DHCP domain name")?;
                if routers.len() > MAX_DNS_SERVERS || dns_servers.len() > MAX_DNS_SERVERS {
                    return Err("DHCP address list exceeds its bound".into());
                }
                if client_mac.is_some_and(|mac| !valid_mac(mac))
                    || assigned_address.is_some_and(|address| !valid_ipv4(address))
                    || requested_address.is_some_and(|address| !valid_ipv4(address))
                    || server_address.is_some_and(|address| !valid_ipv4(address))
                    || routers.iter().any(|address| !valid_ipv4(*address))
                    || dns_servers.iter().any(|address| !valid_ipv4(*address))
                {
                    return Err("DHCP observation contains an unusable address".into());
                }
            }
            Self::NeighborMapping {
                interface,
                address,
                mac_address,
                ..
            } => {
                bounded_text(interface, "neighbor interface")?;
                if !valid_ip(*address) || mac_address.is_some_and(|mac| !valid_mac(mac)) {
                    return Err("neighbor observation contains an unusable address".into());
                }
            }
        }
        Ok(())
    }
}

fn valid_ip(address: IpAddr) -> bool {
    !address.is_unspecified()
        && !address.is_multicast()
        && !matches!(address, IpAddr::V4(address) if address.is_broadcast())
}

fn valid_ipv4(address: Ipv4Addr) -> bool {
    valid_ip(IpAddr::V4(address))
}

fn valid_mac(address: MacAddress) -> bool {
    let bytes = address.bytes();
    bytes != [0; 6] && bytes != [0xff; 6] && bytes[0] & 1 == 0
}

fn optional_text(value: &Option<String>, name: &str) -> Result<(), String> {
    if let Some(value) = value {
        bounded_text(value, name)?;
    }
    Ok(())
}

fn bounded_text(value: &str, name: &str) -> Result<(), String> {
    if value.trim().is_empty()
        || value.len() > MAX_TEXT_BYTES
        || value.chars().any(char::is_control)
    {
        return Err(format!(
            "{name} is empty, oversized, or contains control characters"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unbounded_or_raw_shaped_values() {
        let observed_at = Utc::now();
        let input = PassiveObservationInput {
            observation_id: Uuid::new_v4(),
            source: PassiveSource::Mdns,
            confidence: 75,
            observed_at,
            expires_at: Some(observed_at + chrono::Duration::seconds(120)),
            fact: PassiveFact::MdnsService {
                service_type: "_ssh._tcp.local".into(),
                instance: "x".repeat(MAX_TEXT_BYTES + 1),
                hostname: None,
                addresses: vec![],
                port: Some(22),
                txt_keys: vec![],
                ttl_seconds: 120,
            },
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn rejects_mislabeled_provenance_and_caller_selected_confidence() {
        let mut input = PassiveObservationInput {
            observation_id: Uuid::new_v4(),
            source: PassiveSource::Dhcp,
            confidence: 85,
            observed_at: Utc::now(),
            expires_at: None,
            fact: PassiveFact::NeighborMapping {
                address: "192.0.2.1".parse().unwrap(),
                mac_address: None,
                interface: "eth0".into(),
                state: NeighborState::Incomplete,
            },
        };
        assert!(input.validate().unwrap_err().contains("source"));
        input.source = PassiveSource::KernelNeighbor;
        assert!(input.validate().unwrap_err().contains("confidence"));
        input.confidence = 35;
        input.expires_at = Some(input.observed_at + chrono::Duration::minutes(30));
        assert_eq!(input.validate(), Ok(()));
    }
}

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::server::shared::types::field_definition::{FieldDefinition, FieldType};

/// Scan performance settings. Lives on the discovery entity.
/// All fields have sensible defaults — a discovery with `ScanSettings::default()` behaves
/// identically to how scans worked before this change.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema)]
pub struct ScanSettings {
    /// ARP retry rounds for non-responsive targets (default: 2 = 3 total attempts)
    #[serde(default = "defaults::arp_retries")]
    pub arp_retries: u32,

    /// ARP packets per second (default: 50)
    #[serde(default = "defaults::arp_rate_pps")]
    pub arp_rate_pps: u32,

    /// Port scan probes per second (default: 500)
    #[serde(default = "defaults::scan_rate_pps")]
    pub scan_rate_pps: u32,

    /// Ports scanned concurrently per host (default: 200, clamped 16-1000)
    #[serde(default = "defaults::port_scan_batch_size")]
    pub port_scan_batch_size: usize,

    /// Network interfaces to restrict scanning to (default: empty = all)
    #[serde(default)]
    pub interfaces: Vec<String>,

    /// Whether to probe raw-socket ports 9100-9107 (default: false).
    /// Disabled by default to prevent ghost printing on JetDirect printers.
    #[serde(default)]
    pub probe_raw_socket_ports: bool,

    /// On Windows, use Npcap broadcast ARP instead of SendARP (default: false)
    #[serde(default)]
    pub use_npcap_arp: bool,
}

pub mod defaults {
    pub fn arp_retries() -> u32 {
        2
    }
    pub fn arp_rate_pps() -> u32 {
        50
    }
    pub fn scan_rate_pps() -> u32 {
        500
    }
    pub fn port_scan_batch_size() -> usize {
        200
    }
}

impl ScanSettings {
    pub fn field_definitions() -> Vec<FieldDefinition> {
        // Destructure to enforce completeness — compiler errors if a field is added
        // to ScanSettings but not represented here.
        let Self {
            arp_retries: _,
            arp_rate_pps: _,
            scan_rate_pps: _,
            port_scan_batch_size: _,
            interfaces: _,
            probe_raw_socket_ports: _,
            use_npcap_arp: _,
        } = Self::default();

        vec![
            FieldDefinition {
                id: "scan_rate_pps",
                label: "Port Scan Rate",
                field_type: FieldType::Number,
                placeholder: Some("500"),
                secret: false,
                optional: false,
                help_text: Some(
                    "Probes per second during port scanning. Lower values reduce network impact.",
                ),
                options: None,
                default_value: Some("500"),
                category: Some("Port Scanning"),
            },
            FieldDefinition {
                id: "arp_rate_pps",
                label: "ARP Scan Rate",
                field_type: FieldType::Number,
                placeholder: Some("50"),
                secret: false,
                optional: false,
                help_text: Some(
                    "ARP packets per second during host discovery. Keep low on networks with strict switch policies.",
                ),
                options: None,
                default_value: Some("50"),
                category: Some("ARP"),
            },
            FieldDefinition {
                id: "arp_retries",
                label: "ARP Retries",
                field_type: FieldType::Number,
                placeholder: Some("2"),
                secret: false,
                optional: false,
                help_text: Some(
                    "Additional ARP rounds for non-responsive hosts. Total attempts = retries + 1.",
                ),
                options: None,
                default_value: Some("2"),
                category: Some("ARP"),
            },
            FieldDefinition {
                id: "port_scan_batch_size",
                label: "Port Scan Batch Size",
                field_type: FieldType::Number,
                placeholder: Some("200"),
                secret: false,
                optional: false,
                help_text: Some("Ports scanned concurrently per host. Range: 16-1000."),
                options: None,
                default_value: Some("200"),
                category: Some("Port Scanning"),
            },
            FieldDefinition {
                id: "interfaces",
                label: "Network Interfaces",
                field_type: FieldType::String,
                placeholder: Some("e.g. eth0, enp3s0"),
                secret: false,
                optional: true,
                help_text: Some(
                    "Restrict scanning to specific interfaces. Leave empty to scan all.",
                ),
                options: None,
                default_value: None,
                category: Some("Targets"),
            },
            FieldDefinition {
                id: "probe_raw_socket_ports",
                label: "Probe Raw Socket Ports",
                field_type: FieldType::Boolean,
                placeholder: None,
                secret: false,
                optional: false,
                help_text: Some(
                    "Scan ports 9100-9107 (JetDirect, Prometheus). May cause ghost printing on some printers.",
                ),
                options: None,
                default_value: Some("false"),
                category: Some("Port Scanning"),
            },
            FieldDefinition {
                id: "use_npcap_arp",
                label: "Use Npcap ARP (Windows)",
                field_type: FieldType::Boolean,
                placeholder: None,
                secret: false,
                optional: false,
                help_text: Some(
                    "Use Npcap broadcast ARP instead of Windows SendARP. Requires Npcap installed.",
                ),
                options: None,
                default_value: Some("false"),
                category: Some("ARP"),
            },
        ]
    }
}

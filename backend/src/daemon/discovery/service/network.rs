use crate::daemon::discovery::integration::{
    IntegrationContext, IntegrationRegistry, execute_with_progress_reporting,
};
use crate::daemon::discovery::service::ops::DiscoveryOps;
use crate::daemon::discovery::types::base::DiscoveryCriticalError;
use crate::daemon::utils::arp::{self, ArpScanResult};
use crate::daemon::utils::base::PlatformDaemonUtils;
use crate::daemon::utils::scanner::{
    ScanConcurrencyController, can_arp_scan, scan_endpoints, scan_tcp_ports, scan_udp_ports,
};
use crate::server::credentials::r#impl::mapping::CredentialQueryPayloadDiscriminants;
use crate::server::credentials::r#impl::mapping::{
    DockerProxyQueryCredential, ResolvedCredential, SnmpCredentialMapping,
};
use crate::server::credentials::r#impl::types::CredentialAssignment;
use crate::server::discovery::r#impl::scan_settings::{ScanSettings, defaults};
use crate::server::discovery::r#impl::types::{DiscoveryType, HostNamingFallback};
use crate::server::interfaces::r#impl::base::{Interface, InterfaceBase};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::base::{Service, ServiceMatchBaselineParams};
use crate::server::shared::types::entities::EntitySource;
use crate::server::subnets::r#impl::types::SubnetTypeDiscriminants;
use crate::{
    daemon::utils::base::DaemonUtils,
    server::{
        daemons::r#impl::base::DaemonMode,
        hosts::r#impl::{
            api::{DiscoveryHostRequest, HostResponse},
            base::{Host, HostBase},
        },
        subnets::r#impl::base::Subnet,
    },
};
use anyhow::Error;
use cidr::IpCidr;
use futures::{StreamExt, future::try_join_all};
use mac_address::MacAddress;
use pnet::datalink;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use std::{net::IpAddr, sync::Arc};
use strum::IntoDiscriminant;
use tokio::sync::mpsc as tokio_mpsc;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Per-host data discovered during deep_scan_host().
/// Used by subsequent discovery phases (e.g., Docker container scanning) to link
/// containers to the correct virtualizing service and provide host interfaces.
#[derive(Debug, Clone, Default)]
pub struct DiscoveredHostData {
    pub docker_service_id: Option<Uuid>,
    pub interfaces: Vec<Interface>,
}

/// Grace period to wait for late ARP arrivals after the last deep scan completes
const LATE_ARRIVAL_GRACE_PERIOD: Duration = Duration::from_secs(30);

/// Hard maximum duration for a single discovery run (safety net)
const MAX_DISCOVERY_DURATION: Duration = Duration::from_secs(21600); // 6 hours

/// Maximum interval between progress reports (heartbeat even if progress unchanged)
const MAX_PROGRESS_REPORT_INTERVAL: Duration = Duration::from_secs(30);

// Progress phase weights (must sum to 100)
const PROGRESS_ARP_PHASE: u8 = 30; // 0-30%: ARP discovery
const PROGRESS_DEEP_SCAN_PHASE: u8 = 65; // 30-95%: Deep scanning
const PROGRESS_GRACE_PHASE: u8 = 5; // 95-100%: Grace period

/// Cost of a full port scan per host in centiseconds
const FULL_SCAN_COST_CS: usize = 9000; // ~90 seconds
/// Cost of a light scan per host in centiseconds
const LIGHT_SCAN_COST_CS: usize = 800; // ~8 seconds

#[derive(Default)]
pub struct NetworkScanDiscovery {
    subnet_ids: Option<Vec<Uuid>>,
    host_naming_fallback: HostNamingFallback,
    scan_settings: ScanSettings,
    /// All credential mappings for integration dispatch.
    credential_mappings: Vec<
        crate::server::credentials::r#impl::mapping::CredentialMapping<
            crate::server::credentials::r#impl::mapping::CredentialQueryPayload,
        >,
    >,
    /// SNMP credentials extracted from credential_mappings.
    /// TODO: Remove when SNMP scanning is fully handled by SnmpIntegration.
    snmp_credentials: SnmpCredentialMapping,
    /// Docker credentials indexed by target IP, extracted from credential_mappings.
    /// TODO: Remove when Docker probing is fully handled by DockerIntegration.
    docker_credentials: HashMap<IpAddr, ResolvedCredential<DockerProxyQueryCredential>>,
    /// Precomputed set of ports for light scans (discovery + credential ports)
    light_scan_ports: HashSet<u16>,
}

impl NetworkScanDiscovery {
    pub fn new(
        subnet_ids: Option<Vec<Uuid>>,
        host_naming_fallback: HostNamingFallback,
        snmp_credentials: SnmpCredentialMapping,
        scan_settings: ScanSettings,
        docker_credentials: HashMap<IpAddr, ResolvedCredential<DockerProxyQueryCredential>>,
        credential_mappings: Vec<
            crate::server::credentials::r#impl::mapping::CredentialMapping<
                crate::server::credentials::r#impl::mapping::CredentialQueryPayload,
            >,
        >,
    ) -> Self {
        // Build light scan port set: discovery ports + credential custom ports
        let mut light_scan_ports: HashSet<u16> = Service::all_discovery_ports()
            .iter()
            .filter(|p| p.is_tcp())
            .map(|p| p.number())
            .collect();

        // Add custom ports from DockerProxy credentials
        for cred in docker_credentials.values() {
            light_scan_ports.insert(cred.credential.port);
        }

        // Add SNMP standard ports if SNMP credentials are present
        if snmp_credentials.default_credential.is_some()
            || !snmp_credentials.ip_overrides.is_empty()
        {
            light_scan_ports.insert(161);
            light_scan_ports.insert(1161);
        }

        Self {
            subnet_ids,
            host_naming_fallback,
            scan_settings,
            credential_mappings,
            snmp_credentials,
            docker_credentials,
            light_scan_ports,
        }
    }
}

pub struct DeepScanParams<'a> {
    ip: IpAddr,
    subnet: &'a Subnet,
    mac: Option<MacAddress>,
    cancel: CancellationToken,
    scan_rate_pps: u32,
    port_scan_batch_size: usize,
    gateway_ips: &'a [IpAddr],
    /// Completed cost counter in centiseconds for cost-based progress tracking
    completed_cost: Option<&'a Arc<AtomicUsize>>,
    /// Total cost counter in centiseconds - for non-interfaced hosts, we add to this AFTER
    /// the responsiveness check passes (so only responsive hosts are counted)
    total_cost: Option<&'a Arc<AtomicUsize>>,
    /// Hosts discovered counter - for non-interfaced hosts, we increment AFTER
    /// the responsiveness check passes (so only responsive hosts are counted)
    hosts_discovered: Option<&'a Arc<AtomicUsize>>,
    /// Number of TCP port scan batches expected for this host
    batches_per_host: usize,
    /// Cost of scanning this host in centiseconds (port scan only, no integrations)
    scan_cost_cs: usize,
    /// Shared concurrency controller for graceful FD exhaustion handling
    scan_controller: Arc<ScanConcurrencyController>,
    /// Whether to probe raw-socket ports (9100-9107) during endpoint scanning
    probe_raw_socket_ports: bool,
    /// Host ID from early reporting — reused in final create_host to update rather than duplicate
    early_host_id: Uuid,
    /// Docker credential for this host, if any
    docker_credential: Option<ResolvedCredential<DockerProxyQueryCredential>>,
    /// Whether this is a full 65k port scan (vs light scan with discovery ports only)
    is_full_scan: bool,
    /// Precomputed light scan port set (used when is_full_scan is false)
    light_scan_ports: &'a HashSet<u16>,
    credential_mappings: &'a [crate::server::credentials::r#impl::mapping::CredentialMapping<
        crate::server::credentials::r#impl::mapping::CredentialQueryPayload,
    >],
}

impl NetworkScanDiscovery {
    /// Compute the total integration cost (centiseconds) for a specific IP.
    /// Sums estimated_seconds for each integration that has a credential covering this IP.
    fn compute_integration_cost_for_ip(&self, ip: IpAddr) -> usize {
        self.credential_mappings
            .iter()
            .filter_map(|m| {
                let discriminant: CredentialQueryPayloadDiscriminants = m
                    .default_credential
                    .as_ref()
                    .map(|c| c.into())
                    .or_else(|| m.ip_overrides.first().map(|o| (&o.credential).into()))?;
                let has_cred =
                    m.ip_overrides.iter().any(|o| o.ip == ip) || m.default_credential.is_some();
                if has_cred {
                    let integration = IntegrationRegistry::get(discriminant);
                    Some(integration.estimated_seconds() as usize * 100)
                } else {
                    None
                }
            })
            .sum()
    }

    pub async fn discover_create_subnets(
        &self,
        ops: &DiscoveryOps,
        utils: &PlatformDaemonUtils,
        discovery_type: DiscoveryType,
        cancel: &CancellationToken,
    ) -> Result<Vec<Subnet>, Error> {
        let daemon_id = ops.config_store.get_id().await?;
        let network_id = ops
            .config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network ID not set"))?;

        // Target specific subnets if provided in discovery type
        let subnets = if let Some(subnet_ids) = &self.subnet_ids {
            let all_subnets: Vec<Subnet> = ops
                .api_client
                .get("/api/v1/subnets", "Failed to get subnets")
                .await?;
            all_subnets
                .into_iter()
                .filter(|s| subnet_ids.contains(&s.id))
                .collect()

        // Target all interfaced subnets if not
        } else {
            let interface_filter = ops.config_store.get_interfaces().await?;
            let (_, subnets, _) = utils
                .get_own_interfaces(discovery_type, daemon_id, network_id, &interface_filter)
                .await?;

            // Filter out docker bridge subnets (handled in docker discovery).
            // Size filtering for non-interfaced subnets is done later in
            // scan_and_process_hosts() where subnet_cidr_to_mac is available.
            let subnets: Vec<Subnet> = subnets
                .into_iter()
                .filter(|s| {
                    if s.base.subnet_type.discriminant() == SubnetTypeDiscriminants::DockerBridge {
                        tracing::warn!("Skipping {} with CIDR {}, docker bridge subnets are scanned in docker discovery", s.base.name, s.base.cidr);
                        return false
                    }

                    true
                })
                .collect();
            let subnet_futures = subnets
                .iter()
                .map(|subnet| ops.create_subnet(subnet, cancel));
            try_join_all(subnet_futures).await?
        };

        Ok(subnets)
    }
    pub async fn scan_and_process_hosts(
        &self,
        subnets: Vec<Subnet>,
        cancel: CancellationToken,
        ops: &DiscoveryOps,
        utils: &PlatformDaemonUtils,
    ) -> Result<Vec<(IpAddr, Host, DiscoveredHostData)>, Error> {
        let session = ops.get_session().await?;

        let interface_filter = ops.config_store.get_interfaces().await?;
        let (_, _, subnet_cidr_to_mac) = utils
            .get_own_interfaces(
                ops.discovery_type.clone(),
                session.info.daemon_id,
                session.info.network_id,
                &interface_filter,
            )
            .await?;

        // Filter subnets: skip loopback and subnets exceeding the configurable
        // minimum prefix length. This prevents OOM from bogus large CIDRs.
        let min_prefix = self
            .scan_settings
            .min_subnet_prefix
            .unwrap_or(defaults::min_subnet_prefix());
        let subnets: Vec<Subnet> = subnets
            .into_iter()
            .filter(|s| {
                if s.base.subnet_type.is_loopback() {
                    return false;
                }
                if s.base.cidr.network_length() < min_prefix {
                    tracing::warn!(
                        subnet = %s.base.name,
                        cidr = %s.base.cidr,
                        min_prefix,
                        "Skipping subnet larger than /{}, scanning would use too much memory",
                        min_prefix
                    );
                    return false;
                }
                true
            })
            .collect();

        // Get scan settings from discovery request, falling back to defaults
        let use_npcap = self.scan_settings.use_npcap_arp;
        let arp_retries = self
            .scan_settings
            .arp_retries
            .unwrap_or(defaults::arp_retries());
        let arp_rate_pps = self
            .scan_settings
            .arp_rate_pps
            .unwrap_or(defaults::arp_rate_pps());
        let scan_rate_pps = self
            .scan_settings
            .scan_rate_pps
            .unwrap_or(defaults::scan_rate_pps());
        let port_scan_batch_size = self
            .scan_settings
            .port_scan_batch_size
            .unwrap_or(defaults::port_scan_batch_size())
            .clamp(16, 1000);

        // Check ARP capability once before partitioning
        let arp_available = can_arp_scan(use_npcap);

        // Partition subnets (not IPs) into interfaced vs non-interfaced.
        // IPs are generated per-subnet at point of use to avoid allocating a
        // single Vec with every IP across all subnets (which OOMs on bogus CIDRs).
        let (interfaced_subnets, non_interfaced_subnets): (Vec<_>, Vec<_>) = if arp_available {
            subnets.into_iter().partition(|s| {
                subnet_cidr_to_mac
                    .get(&s.base.cidr)
                    .and_then(|m| *m)
                    .is_some()
            })
        } else {
            (Vec::new(), subnets)
        };

        // Compute IP counts from prefix lengths without materializing all IPs
        let count_ips = |subnets: &[Subnet]| -> u64 {
            subnets
                .iter()
                .map(|s| 1u64 << (32 - s.base.cidr.network_length() as u64))
                .sum()
        };
        let interfaced_ip_count = count_ips(&interfaced_subnets);
        let non_interfaced_ip_count = count_ips(&non_interfaced_subnets);
        let total_ips = interfaced_ip_count + non_interfaced_ip_count;

        // Calculate estimated ARP duration for progress reporting
        let arp_target_count = interfaced_ip_count;
        let total_rounds = 1 + arp_retries as u64;
        let send_time_per_round_secs = arp_target_count / arp_rate_pps.max(1) as u64;
        let estimated_arp_duration = Duration::from_secs(
            total_rounds * (send_time_per_round_secs + arp::ROUND_WAIT.as_secs())
                + arp::POST_SCAN_RECEIVE.as_secs(),
        );
        let pipeline_start = Instant::now();

        tracing::info!(
            total_ips,
            interfaced_ips = interfaced_ip_count,
            non_interfaced_ips = non_interfaced_ip_count,
            estimated_arp_secs = estimated_arp_duration.as_secs(),
            arp_method = if cfg!(target_family = "windows") && !use_npcap {
                "SendARP"
            } else {
                "Broadcast"
            },
            "Starting continuous discovery pipeline"
        );

        ops.report_progress(0).await?;

        let arp_subnet_count = interfaced_subnets.len();

        // Use the port batch size from the coordinated calculation
        let effective_batch_size = port_scan_batch_size;

        // Calculate deep scan concurrency based on FDs available after ARP
        let mut deep_scan_concurrency =
            utils.get_optimal_deep_scan_concurrency(effective_batch_size, arp_subnet_count)?;

        // Create shared concurrency controller for graceful degradation
        let scan_controller = ScanConcurrencyController::new(effective_batch_size);

        let gateway_ips = utils.get_own_routing_table_gateway_ips().await?;

        // Create async channel for discovered hosts
        // Buffer size allows ARP to run ahead while deep scanning catches up
        let (host_tx, mut host_rx) =
            tokio_mpsc::channel::<(IpAddr, Subnet, Option<MacAddress>)>(256);

        // Start ARP scanning for interfaced subnets — build target IPs per-subnet
        if !interfaced_subnets.is_empty() {
            let mut subnet_to_ips: HashMap<IpCidr, (Subnet, Vec<std::net::Ipv4Addr>)> =
                HashMap::new();
            for subnet in &interfaced_subnets {
                let entry = subnet_to_ips
                    .entry(subnet.base.cidr)
                    .or_insert_with(|| (subnet.clone(), Vec::new()));
                for ip in self.determine_scan_order(&subnet.base.cidr) {
                    if let IpAddr::V4(ipv4) = ip {
                        entry.1.push(ipv4);
                    }
                }
            }

            tracing::info!(
                subnets = subnet_to_ips.len(),
                total_ips = interfaced_ip_count,
                arp_retries,
                arp_rate_pps,
                cidrs = ?subnet_to_ips.keys().map(|c| c.to_string()).collect::<Vec<_>>(),
                "Starting ARP discovery"
            );

            // Start ARP scan for each subnet and forward results to async channel
            for (cidr, (subnet, target_ips)) in subnet_to_ips {
                if cancel.is_cancelled() {
                    return Err(Error::msg("Discovery session was cancelled"));
                }

                let subnet_mac = subnet_cidr_to_mac.get(&cidr).and_then(|m| *m);

                let Some(source_mac) = subnet_mac else {
                    tracing::warn!(cidr = %cidr, "No MAC address found for subnet, skipping ARP scan");
                    continue;
                };

                // Find the network interface for this subnet
                // Match by both MAC and having an IP in the target subnet to handle
                // bridge setups where physical and bridge interfaces share the same MAC
                let pnet_source_mac = pnet::util::MacAddr::from(source_mac.bytes());
                let interface = datalink::interfaces().into_iter().find(|iface| {
                    iface.mac.unwrap_or_default() == pnet_source_mac
                        && iface.ips.iter().any(|ip| cidr.contains(&ip.ip()))
                });

                let Some(interface) = interface else {
                    tracing::warn!(mac = %source_mac, "No interface found for MAC, skipping ARP scan");
                    continue;
                };

                // Get an IPv4 address from this interface (prefer one on the target subnet)
                let source_ipv4 = interface
                    .ips
                    .iter()
                    .filter_map(|ip_net| match ip_net.ip() {
                        IpAddr::V4(ip) => Some(ip),
                        IpAddr::V6(_) => None,
                    })
                    .find(|ip| cidr.contains(&IpAddr::V4(*ip)))
                    .or_else(|| {
                        interface.ips.iter().find_map(|ip_net| match ip_net.ip() {
                            IpAddr::V4(ip) => Some(ip),
                            IpAddr::V6(_) => None,
                        })
                    });

                let Some(source_ipv4) = source_ipv4 else {
                    tracing::warn!(
                        interface = %interface.name,
                        cidr = %cidr,
                        "No IPv4 address found on interface, skipping ARP scan"
                    );
                    continue;
                };

                let target_count = target_ips.len();
                tracing::debug!(
                    cidr = %cidr,
                    interface = %interface.name,
                    source_ip = %source_ipv4,
                    source_mac = %source_mac,
                    targets = target_count,
                    arp_rate_pps,
                    "Starting ARP scan"
                );

                match arp::scan_subnet(
                    &interface,
                    source_ipv4,
                    source_mac,
                    target_ips,
                    use_npcap,
                    arp_retries,
                    arp_rate_pps,
                ) {
                    Ok(arp_rx) => {
                        // Spawn a task to forward ARP results to the async channel
                        // Use spawn_blocking since std::sync::mpsc::recv_timeout is blocking
                        let host_tx = host_tx.clone();
                        let subnet = subnet.clone();

                        // Use a background thread for the blocking recv, forward via channel.
                        // Hard timeout prevents infinite hangs if the ARP receiver thread
                        // gets stuck (e.g., on bridge interfaces with continuous traffic).
                        std::thread::spawn(move || {
                            let forwarder_start = Instant::now();
                            let forwarder_timeout = Duration::from_secs(600); // 10 minutes
                            let mut forwarded = 0u64;
                            loop {
                                if forwarder_start.elapsed() >= forwarder_timeout {
                                    tracing::warn!(
                                        cidr = %cidr,
                                        forwarded,
                                        elapsed_secs = forwarder_start.elapsed().as_secs(),
                                        "ARP forwarder hit timeout, forcing exit"
                                    );
                                    break;
                                }

                                match arp_rx.recv_timeout(Duration::from_millis(100)) {
                                    Ok(ArpScanResult { ip, mac }) => {
                                        // Use blocking_send since we're in a std thread
                                        if host_tx
                                            .blocking_send((
                                                IpAddr::V4(ip),
                                                subnet.clone(),
                                                Some(mac),
                                            ))
                                            .is_err()
                                        {
                                            // Receiver dropped, stop forwarding
                                            break;
                                        }
                                        forwarded += 1;
                                    }
                                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                                }
                            }
                            tracing::debug!(
                                cidr = %cidr,
                                forwarded,
                                "ARP forwarder completed"
                            );
                        });
                    }
                    Err(e) => {
                        if DiscoveryCriticalError::is_critical_error(e.to_string()) {
                            tracing::error!(cidr = %cidr, error = %e, "Critical error starting ARP scan");
                        } else {
                            tracing::warn!(cidr = %cidr, error = %e, "ARP scan failed to start");
                        }
                    }
                }
            }
        }

        // Send all non-interfaced IPs directly to deep scanner (no discovery phase).
        // Key insight: ARP filters to responsive hosts before expensive port scanning.
        // For non-interfaced subnets where ARP isn't possible, just deep scan all IPs
        // directly - we're going to port scan them anyway.
        if !non_interfaced_subnets.is_empty() {
            tracing::info!(
                count = non_interfaced_ip_count,
                "Queuing non-interfaced IPs for deep scan (no ARP available)"
            );

            // Generate IPs per-subnet (each bounded by min_prefix filter)
            let non_interfaced_ips: Vec<(IpAddr, Subnet)> = non_interfaced_subnets
                .iter()
                .flat_map(|subnet| {
                    self.determine_scan_order(&subnet.base.cidr)
                        .map(move |ip| (ip, subnet.clone()))
                })
                .collect();

            let host_tx = host_tx.clone();
            tokio::spawn(async move {
                for (ip, subnet) in non_interfaced_ips {
                    if host_tx.send((ip, subnet, None)).await.is_err() {
                        break; // Receiver dropped
                    }
                }
            });
        }

        // Drop our copy of the sender so the channel closes when all forwarders are done
        drop(host_tx);

        // =============================================================
        // CONTINUOUS PIPELINE: Deep scan hosts as they arrive
        // =============================================================
        tracing::info!(
            deep_scan_concurrency,
            grace_period_secs = LATE_ARRIVAL_GRACE_PERIOD.as_secs(),
            "Deep scanning hosts as they are discovered"
        );

        let hosts_discovered = Arc::new(AtomicUsize::new(0));
        let hosts_scanned = Arc::new(AtomicUsize::new(0));
        let last_activity = Arc::new(std::sync::Mutex::new(Instant::now()));
        let mut results: Vec<(IpAddr, Host, DiscoveredHostData)> = Vec::new();

        // Batch-level progress tracking for smoother UX
        // TCP port scanning is the bulk of deep scan work
        let is_full_scan = self.scan_settings.is_full_scan;
        let scan_port_count = if is_full_scan {
            65535_usize
        } else {
            self.light_scan_ports.len()
        };
        let batches_per_host = scan_port_count.div_ceil(effective_batch_size);
        let scan_cost_cs = if is_full_scan {
            FULL_SCAN_COST_CS
        } else {
            LIGHT_SCAN_COST_CS
        };
        let total_cost = Arc::new(AtomicUsize::new(0));
        let completed_cost = Arc::new(AtomicUsize::new(0));

        // Collect hosts into a stream and process with concurrency limit
        // Use trait objects to allow spawning from different code paths
        #[allow(clippy::type_complexity)]
        let mut pending_scans: futures::stream::FuturesUnordered<
            std::pin::Pin<
                Box<
                    dyn std::future::Future<Output = Option<(IpAddr, Host, DiscoveredHostData)>>
                        + Send,
                >,
            >,
        > = futures::stream::FuturesUnordered::new();
        let mut channel_closed = false;
        let mut last_progress_report = 0u8;
        let mut last_progress_time = Instant::now();
        let mut deep_scan_started_at: Option<Instant> = None;

        // Buffer for hosts waiting to be scanned when at concurrency limit
        let mut pending_hosts: Vec<(IpAddr, Subnet, Option<MacAddress>)> = Vec::new();

        // Use interval instead of sleep - interval persists across select iterations
        // whereas sleep creates a new future each time and gets dropped when other branches fire
        let mut progress_ticker = tokio::time::interval(Duration::from_secs(1));

        // Helper to calculate phase-weighted progress
        // Note: counters passed by value to avoid borrowing issues in closure
        let calculate_progress = |channel_closed: bool,
                                  has_pending_scans: bool,
                                  grace_elapsed: Duration,
                                  total_cost_val: usize,
                                  completed_cost_val: usize,
                                  hosts_discovered_val: usize,
                                  hosts_scanned_val: usize|
         -> u8 {
            if !channel_closed {
                // ARP phase (0-30%): Based on elapsed time vs estimated duration
                let arp_elapsed = pipeline_start.elapsed();
                let arp_progress = if estimated_arp_duration.as_secs() > 0 {
                    (arp_elapsed.as_secs_f64() / estimated_arp_duration.as_secs_f64()).min(1.0)
                } else {
                    1.0
                };
                (arp_progress * PROGRESS_ARP_PHASE as f64) as u8
            } else if total_cost_val > 0
                && (completed_cost_val < total_cost_val || has_pending_scans)
            {
                // Deep scan phase (30-95%): Based on batch completion ratio for smooth progress
                let scan_progress = completed_cost_val as f64 / total_cost_val as f64;
                PROGRESS_ARP_PHASE + (scan_progress * PROGRESS_DEEP_SCAN_PHASE as f64) as u8
            } else if has_pending_scans && total_cost_val == 0 && hosts_discovered_val > 0 {
                // Channel closed but no batch info yet - use host-level progress
                // to avoid getting stuck at 30% when batches haven't been registered
                let host_progress =
                    (hosts_scanned_val as f64 / hosts_discovered_val as f64).min(1.0);
                PROGRESS_ARP_PHASE + (host_progress * PROGRESS_DEEP_SCAN_PHASE as f64) as u8
            } else if has_pending_scans {
                // Deep scan with no batch info yet - show minimal progress
                PROGRESS_ARP_PHASE
            } else {
                // Grace period phase (95-100%): Based on grace period elapsed
                let grace_progress = (grace_elapsed.as_secs_f64()
                    / LATE_ARRIVAL_GRACE_PERIOD.as_secs_f64())
                .min(1.0);
                PROGRESS_ARP_PHASE
                    + PROGRESS_DEEP_SCAN_PHASE
                    + (grace_progress * PROGRESS_GRACE_PHASE as f64) as u8
            }
        };

        let mut early_reported_hosts: HashMap<
            IpAddr,
            tokio::task::JoinHandle<Result<Uuid, Error>>,
        > = HashMap::new();

        loop {
            tokio::select! {
                // Try to receive new hosts from the channel
                host = host_rx.recv(), if !channel_closed => {
                    match host {
                        Some((ip, subnet, mac)) => {
                            // Only count ARP-confirmed hosts immediately.
                            // Non-interfaced hosts are counted after responsiveness
                            // check passes in deep_scan_host().
                            if mac.is_some() {
                                hosts_discovered.fetch_add(1, Ordering::Relaxed);
                            }
                            *last_activity.lock().unwrap() = Instant::now();

                            // Early-report a minimal host so the UI shows it immediately.
                            // Only for interfaced hosts (mac.is_some()) — they're confirmed
                            // responsive via ARP. Non-interfaced hosts must pass the TCP
                            // responsiveness check in deep_scan_host() first.
                            if mac.is_some()
                                && let std::collections::hash_map::Entry::Vacant(e) = early_reported_hosts.entry(ip)
                            {
                                let early_subnet = subnet.clone();
                                let early_cancel = cancel.clone();
                                let early_entity_buffer = ops.entity_buffer.clone();
                                let early_config_store = ops.config_store.clone();
                                let early_api_client = ops.api_client.clone();
                                let early_handle = tokio::spawn(async move {
                                    let host = Host::new(HostBase {
                                        name: ip.to_string(),
                                        network_id: early_subnet.base.network_id,
                                        source: EntitySource::Discovery { metadata: vec![] },
                                        ..Default::default()
                                    });
                                    let host_id = host.id;
                                    let interface = Interface::new(InterfaceBase {
                                        network_id: early_subnet.base.network_id,
                                        host_id: Uuid::nil(),
                                        name: None,
                                        subnet_id: early_subnet.id,
                                        ip_address: ip,
                                        mac_address: mac,
                                        position: 0,
                                    });
                                    let request = DiscoveryHostRequest {
                                        host,
                                        interfaces: vec![interface],
                                        ports: vec![],
                                        services: vec![],
                                        if_entries: vec![],
                                    };
                                    early_entity_buffer.push_host(request.clone()).await;
                                    let mode = early_config_store.get_mode().await?;
                                    match mode {
                                        DaemonMode::DaemonPoll => {
                                            let _response: HostResponse = early_api_client
                                                .post("/api/v1/hosts/discovery", &request, "Failed to create early host")
                                                .await?;
                                            Ok(host_id)
                                        }
                                        DaemonMode::ServerPoll => {
                                            let _actual = early_entity_buffer
                                                .await_host(&host_id, Duration::from_secs(120), &early_cancel)
                                                .await
                                                .ok_or_else(|| anyhow::anyhow!("Timeout waiting for early host creation"))?;
                                            Ok(host_id)
                                        }
                                    }
                                });
                                e.insert(early_handle);
                                tokio::time::sleep(Duration::from_millis(100)).await;
                            }

                            // Spawn deep scan if under concurrency limit, otherwise buffer
                            if pending_scans.len() < deep_scan_concurrency {
                                let cancel = cancel.clone();
                                let gateway_ips = gateway_ips.clone();
                                let hosts_scanned = hosts_scanned.clone();
                                let last_activity = last_activity.clone();
                                let completed_cost = completed_cost.clone();
                                let total_cost = total_cost.clone();
                                let hosts_discovered = hosts_discovered.clone();
                                let scan_controller = scan_controller.clone();

                                // Only count batches for hosts with MAC (known responsive from ARP).
                                // Non-interfaced hosts will have batches counted AFTER responsiveness check.
                                if mac.is_some() {
                                    let integration_cost = self.compute_integration_cost_for_ip(ip);
                                    total_cost.fetch_add(scan_cost_cs + integration_cost, Ordering::Relaxed);
                                }
                                let docker_credential = self.docker_credentials.get(&ip).cloned();
                                let probe_raw_socket_ports = self.scan_settings.probe_raw_socket_ports;
                                let light_scan_ports = self.light_scan_ports.clone();
                                let early_host_handle = early_reported_hosts.remove(&ip);
                                pending_scans.push(Box::pin(async move {
                                    let early_host_id = match early_host_handle {
                                        Some(handle) => match handle.await {
                                            Ok(Ok(id)) => id,
                                            _ => Uuid::new_v4(),
                                        },
                                        None => Uuid::new_v4(),
                                    };

                                    let result = self
                                        .deep_scan_host(DeepScanParams {
                                            ip,
                                            subnet: &subnet,
                                            mac,
                                            cancel,
                                            scan_rate_pps,
                                            port_scan_batch_size: effective_batch_size,
                                            gateway_ips: &gateway_ips,
                                            completed_cost: Some(&completed_cost),
                                            total_cost: Some(&total_cost),
                                            hosts_discovered: Some(&hosts_discovered),
                                            batches_per_host,
                                            scan_cost_cs,
                                            scan_controller,
                                            probe_raw_socket_ports,
                                            early_host_id,
                                            docker_credential,
                                            is_full_scan,
                                            light_scan_ports: &light_scan_ports,
                                            credential_mappings: &self.credential_mappings,
                                        }, ops, utils)
                                        .await;

                                    hosts_scanned.fetch_add(1, Ordering::Relaxed);
                                    *last_activity.lock().unwrap() = Instant::now();

                                    match result {
                                        Ok(Some(host)) => Some(host),
                                        Ok(None) => None,
                                        Err(e) => {
                                            if DiscoveryCriticalError::is_critical_error(e.to_string()) {
                                                tracing::error!(ip = %ip, error = %e, "Critical error in deep scan");
                                            } else {
                                                tracing::warn!(ip = %ip, error = %e, "Deep scan failed");
                                            }
                                            None
                                        }
                                    }
                                }));
                            } else {
                                // Only count batches for hosts with MAC (known responsive from ARP).
                                // Non-interfaced hosts will have batches counted AFTER responsiveness check.
                                if mac.is_some() {
                                    let integration_cost = self.compute_integration_cost_for_ip(ip);
                                    total_cost.fetch_add(scan_cost_cs + integration_cost, Ordering::Relaxed);
                                }
                                pending_hosts.push((ip, subnet, mac));
                            }
                        }
                        None => {
                            channel_closed = true;

                            tracing::info!(
                                hosts_discovered = hosts_discovered.load(Ordering::Relaxed),
                                pending_scans = pending_scans.len(),
                                pending_hosts = pending_hosts.len(),
                                total_cost = total_cost.load(Ordering::Relaxed),
                                completed_cost = completed_cost.load(Ordering::Relaxed),
                                elapsed_secs = pipeline_start.elapsed().as_secs(),
                                "Host discovery channel closed, transitioning to deep scan phase"
                            );

                            // ARP complete - recalculate concurrency without ARP FD reservation
                            // Those FDs (2 per subnet) are now available for deep scanning
                            if let Ok(new_concurrency) = utils.get_optimal_deep_scan_concurrency(
                                effective_batch_size,
                                0, // No more ARP channels open
                            ) && new_concurrency > deep_scan_concurrency {
                                tracing::info!(
                                    old = deep_scan_concurrency,
                                    new = new_concurrency,
                                    "Increasing deep scan concurrency"
                                );
                                deep_scan_concurrency = new_concurrency;
                            }
                        }
                    }
                }

                // Collect completed deep scans and spawn buffered hosts
                Some(result) = pending_scans.next(), if !pending_scans.is_empty() => {
                    if let Some(host) = result {
                        results.push(host);
                    }

                    // Spawn next buffered host if available
                    // Note: batches only counted for MAC hosts when buffered; non-MAC hosts
                    // have batches counted in deep_scan_host after responsiveness check
                    if let Some((ip, subnet, mac)) = pending_hosts.pop() {
                        let cancel = cancel.clone();
                        let gateway_ips = gateway_ips.clone();
                        let hosts_scanned = hosts_scanned.clone();
                        let last_activity = last_activity.clone();
                        let completed_cost = completed_cost.clone();
                        let total_cost = total_cost.clone();
                        let hosts_discovered = hosts_discovered.clone();
                        let docker_credential = self.docker_credentials.get(&ip).cloned();
                        let scan_controller = scan_controller.clone();
                        let probe_raw_socket_ports = self.scan_settings.probe_raw_socket_ports;
                        let light_scan_ports = self.light_scan_ports.clone();
                        let early_host_handle = early_reported_hosts.remove(&ip);

                        pending_scans.push(Box::pin(async move {
                            let early_host_id = match early_host_handle {
                                Some(handle) => match handle.await {
                                    Ok(Ok(id)) => id,
                                    _ => Uuid::new_v4(),
                                },
                                None => Uuid::new_v4(),
                            };

                            let result = self
                                .deep_scan_host(DeepScanParams {
                                    ip,
                                    subnet: &subnet,
                                    mac,
                                    cancel,
                                    scan_rate_pps,
                                    port_scan_batch_size: effective_batch_size,
                                    gateway_ips: &gateway_ips,
                                    completed_cost: Some(&completed_cost),
                                    total_cost: Some(&total_cost),
                                    hosts_discovered: Some(&hosts_discovered),
                                    batches_per_host,
                                    scan_cost_cs,
                                    scan_controller,
                                    probe_raw_socket_ports,
                                    early_host_id,
                                    docker_credential,
                                    is_full_scan,
                                    light_scan_ports: &light_scan_ports,
                                    credential_mappings: &self.credential_mappings,
                                }, ops, utils)
                                .await;

                            hosts_scanned.fetch_add(1, Ordering::Relaxed);
                            *last_activity.lock().unwrap() = Instant::now();

                            match result {
                                Ok(Some(host)) => Some(host),
                                Ok(None) => None,
                                Err(e) => {
                                    if DiscoveryCriticalError::is_critical_error(e.to_string()) {
                                        tracing::error!(ip = %ip, error = %e, "Critical error in deep scan");
                                    } else {
                                        tracing::warn!(ip = %ip, error = %e, "Deep scan failed");
                                    }
                                    None
                                }
                            }
                        }));
                    }
                }

                // Periodic progress update and grace period check
                _ = progress_ticker.tick() => {
                    let has_pending = !pending_scans.is_empty() || !pending_hosts.is_empty();
                    let grace_elapsed = last_activity.lock().unwrap().elapsed();
                    let total_cost_val = total_cost.load(Ordering::Relaxed);
                    let completed_cost_val = completed_cost.load(Ordering::Relaxed);
                    let hosts_discovered_val = hosts_discovered.load(Ordering::Relaxed);
                    let hosts_scanned_val = hosts_scanned.load(Ordering::Relaxed);

                    // Calculate and report progress (only if changed)
                    let progress = calculate_progress(
                        channel_closed,
                        has_pending,
                        grace_elapsed,
                        total_cost_val,
                        completed_cost_val,
                        hosts_discovered_val,
                        hosts_scanned_val,
                    );

                    // Update estimation atomics on the session
                    if let Ok(session) = ops.get_session().await {
                        session.hosts_discovered.store(hosts_discovered_val as u32, Ordering::Relaxed);

                        if channel_closed && hosts_scanned_val > 0 {
                            // Host-based estimation: uses actual per-host completion time
                            // which includes TCP + endpoints + SNMP + host creation — the
                            // real bottleneck, not just TCP port scanning batches.
                            let started = deep_scan_started_at.get_or_insert(Instant::now());
                            let deep_scan_elapsed = started.elapsed();
                            let time_per_host = deep_scan_elapsed.as_secs_f64() / hosts_scanned_val as f64;
                            let remaining_hosts = hosts_discovered_val.saturating_sub(hosts_scanned_val);
                            let remaining_secs = (remaining_hosts as f64 * time_per_host) as u32
                                + LATE_ARRIVAL_GRACE_PERIOD.as_secs() as u32;
                            session.estimated_remaining_secs.store(remaining_secs, Ordering::Relaxed);
                        } else if completed_cost_val > 0 {
                            // ARP phase still active — fall back to cost-based estimation
                            let started = deep_scan_started_at.get_or_insert(Instant::now());
                            let deep_scan_elapsed = started.elapsed();
                            let time_per_cost_unit = deep_scan_elapsed.as_secs_f64() / completed_cost_val as f64;
                            let remaining_cost = total_cost_val.saturating_sub(completed_cost_val);
                            let remaining_secs = (remaining_cost as f64 * time_per_cost_unit * 1.2) as u32
                                + LATE_ARRIVAL_GRACE_PERIOD.as_secs() as u32;
                            session.estimated_remaining_secs.store(remaining_secs, Ordering::Relaxed);
                        }
                    }

                    // Report progress if it changed OR if enough time has passed (heartbeat)
                    let time_since_last_report = last_progress_time.elapsed();
                    if progress != last_progress_report || time_since_last_report >= MAX_PROGRESS_REPORT_INTERVAL {
                        last_progress_report = progress;
                        last_progress_time = Instant::now();
                        let _ = ops.report_progress(progress.min(99)).await;
                    }

                    // Check grace period expiry
                    if channel_closed && !has_pending && grace_elapsed >= LATE_ARRIVAL_GRACE_PERIOD {
                            tracing::debug!(
                                elapsed_secs = grace_elapsed.as_secs(),
                                "Grace period expired, ending discovery"
                            );
                            break;
                    }
                }
            }

            // Check for cancellation
            if cancel.is_cancelled() {
                tracing::info!("Discovery cancelled by user");
                return Err(Error::msg("Discovery session was cancelled"));
            }

            // Global timeout safety net
            if pipeline_start.elapsed() >= MAX_DISCOVERY_DURATION {
                tracing::error!(
                    elapsed_secs = pipeline_start.elapsed().as_secs(),
                    hosts_discovered = hosts_discovered.load(Ordering::Relaxed),
                    hosts_scanned = hosts_scanned.load(Ordering::Relaxed),
                    pending_scans = pending_scans.len(),
                    pending_hosts = pending_hosts.len(),
                    channel_closed,
                    "Discovery hit global timeout, forcing completion"
                );
                break;
            }

            // Exit when channel closed, no pending scans/hosts, and grace period expired
            if channel_closed && pending_scans.is_empty() && pending_hosts.is_empty() {
                let elapsed = last_activity.lock().unwrap().elapsed();

                if elapsed >= LATE_ARRIVAL_GRACE_PERIOD {
                    break;
                }

                // Log status while waiting
                let discovered = hosts_discovered.load(Ordering::Relaxed);
                if discovered > 0 {
                    tracing::debug!(
                        discovered,
                        scanned = hosts_scanned.load(Ordering::Relaxed),
                        results = results.len(),
                        grace_remaining_secs = (LATE_ARRIVAL_GRACE_PERIOD - elapsed).as_secs(),
                        "Waiting for late arrivals"
                    );
                }
            }
        }

        ops.report_progress(100).await?;

        let discovered = hosts_discovered.load(Ordering::Relaxed);
        tracing::info!(
            hosts_discovered = discovered,
            hosts_scanned = hosts_scanned.load(Ordering::Relaxed),
            results = results.len(),
            elapsed_secs = pipeline_start.elapsed().as_secs(),
            "Discovery pipeline complete"
        );

        Ok(results)
    }

    async fn deep_scan_host(
        &self,
        params: DeepScanParams<'_>,
        ops: &DiscoveryOps,
        utils: &PlatformDaemonUtils,
    ) -> Result<Option<(IpAddr, Host, DiscoveredHostData)>, Error> {
        let DeepScanParams {
            ip,
            subnet,
            mac,
            cancel,
            scan_rate_pps,
            port_scan_batch_size,
            gateway_ips,
            completed_cost,
            total_cost,
            hosts_discovered,
            batches_per_host,
            scan_cost_cs,
            scan_controller,
            probe_raw_socket_ports,
            early_host_id,
            docker_credential,
            is_full_scan,
            light_scan_ports,
            credential_mappings,
        } = params;

        if cancel.is_cancelled() {
            return Err(Error::msg("Discovery was cancelled"));
        }

        // Use fixed batch size, limited by scan controller if FD exhaustion has occurred
        let effective_batch_size = port_scan_batch_size.min(scan_controller.batch_size());

        // For non-interfaced hosts (no MAC from ARP), check responsiveness first.
        // This avoids full 65k port scans on hosts that aren't online.
        let mut responsiveness_ports: HashSet<u16> = HashSet::new();
        if mac.is_none() {
            let discovery_ports: Vec<u16> = Service::all_discovery_ports()
                .iter()
                .filter(|p| p.is_tcp())
                .map(|p| p.number())
                .collect();

            tracing::debug!(
                ip = %ip,
                ports = discovery_ports.len(),
                "Checking responsiveness (non-interfaced host)"
            );

            let responsive_ports = scan_tcp_ports(
                ip,
                cancel.clone(),
                effective_batch_size,
                scan_rate_pps,
                discovery_ports,
                scan_controller.clone(),
            )
            .await?;

            if responsive_ports.is_empty() {
                tracing::debug!(ip = %ip, "Host unresponsive, skipping deep scan");
                return Ok(None);
            }

            // Host is responsive - NOW we count it in hosts_discovered and total_cost
            // This ensures only responsive hosts contribute to progress calculation
            if let Some(discovered) = hosts_discovered {
                discovered.fetch_add(1, Ordering::Relaxed);
            }
            if let Some(total) = total_cost {
                // Compute integration cost from credential mappings for this IP
                let integration_cost_cs: usize = credential_mappings
                    .iter()
                    .filter_map(|m| {
                        let discriminant: CredentialQueryPayloadDiscriminants = m
                            .default_credential
                            .as_ref()
                            .map(|c| c.into())
                            .or_else(|| m.ip_overrides.first().map(|o| (&o.credential).into()))?;
                        let has_cred = m.ip_overrides.iter().any(|o| o.ip == ip)
                            || m.default_credential.is_some();
                        if has_cred {
                            let integration = IntegrationRegistry::get(discriminant);
                            Some(integration.estimated_seconds() as usize * 100)
                        } else {
                            None
                        }
                    })
                    .sum();
                total.fetch_add(scan_cost_cs + integration_cost_cs, Ordering::Relaxed);
            }

            tracing::debug!(
                ip = %ip,
                open_ports = responsive_ports.len(),
                "Host responsive, proceeding with deep scan"
            );

            // Track discovered ports so we don't re-scan them
            responsiveness_ports.extend(responsive_ports.iter().map(|(p, _)| p.number()));
        }

        let remaining_tcp_ports: Vec<u16> = if is_full_scan {
            (1..=65535)
                .filter(|p| !responsiveness_ports.contains(p))
                .collect()
        } else {
            // Light scan: only discovery ports + credential custom ports
            light_scan_ports
                .iter()
                .copied()
                .filter(|p| !responsiveness_ports.contains(p))
                .collect()
        };

        tracing::debug!(
            ip = %ip,
            is_full_scan,
            responsiveness_ports = responsiveness_ports.len(),
            remaining_ports = remaining_tcp_ports.len(),
            effective_batch_size,
            "Starting deep scan"
        );

        // Scan in batches with rate limiting and graceful degradation
        let mut all_tcp_ports = Vec::new();
        for chunk in remaining_tcp_ports.chunks(effective_batch_size) {
            if cancel.is_cancelled() {
                return Err(Error::msg("Discovery was cancelled"));
            }

            let open_ports = scan_tcp_ports(
                ip,
                cancel.clone(),
                effective_batch_size,
                scan_rate_pps,
                chunk.to_vec(),
                scan_controller.clone(),
            )
            .await?;
            all_tcp_ports.extend(open_ports);

            // Update cost-based progress: each batch contributes a fraction of scan_cost_cs
            if let Some(counter) = completed_cost {
                let cost_per_batch = if batches_per_host > 0 {
                    scan_cost_cs / batches_per_host
                } else {
                    0
                };
                counter.fetch_add(cost_per_batch, Ordering::Relaxed);
            }
        }

        let use_https_ports: HashMap<u16, bool> = all_tcp_ports
            .iter()
            .map(|(p, h)| (p.number(), *h))
            .collect();
        let mut open_ports: Vec<PortType> = all_tcp_ports.iter().map(|(p, _)| *p).collect();

        // Merge responsiveness check discovered ports (for non-interfaced hosts)
        for port_num in responsiveness_ports {
            let port = PortType::new_tcp(port_num);
            if !open_ports.contains(&port) {
                open_ports.push(port);
            }
        }
        open_ports.sort_by_key(|p| (p.number(), p.protocol()));
        open_ports.dedup();

        // Non-credentialed UDP scanning (DNS, NTP, DHCP, BACnet).
        // SNMP probing is now handled by SnmpIntegration.probe() below.
        let udp_ports = scan_udp_ports(
            ip,
            cancel.clone(),
            effective_batch_size,
            scan_rate_pps,
            subnet.base.cidr,
            gateway_ips.to_vec(),
            &[], // No SNMP credentials — SNMP probing handled by integration
        )
        .await?;
        open_ports.extend(udp_ports);

        // --- Integration probes ---
        // Replace inline SNMP UDP probing and Docker client probing with generic dispatch.
        // Each integration's probe() checks connectivity and returns a ClientProbe for service matching.
        use crate::daemon::discovery::integration::ProbeContext;
        let mut client_responses: HashMap<
            crate::server::services::r#impl::patterns::ClientProbe,
            Vec<PortType>,
        > = HashMap::new();
        let mut probe_handles: HashMap<
            CredentialQueryPayloadDiscriminants,
            Box<dyn std::any::Any + Send + Sync>,
        > = HashMap::new();
        let mut working_credential_ids: HashMap<
            CredentialQueryPayloadDiscriminants,
            (
                Uuid,
                crate::server::credentials::r#impl::mapping::CredentialQueryPayload,
            ),
        > = HashMap::new();

        for mapping in credential_mappings {
            let discriminant: Option<CredentialQueryPayloadDiscriminants> = mapping
                .default_credential
                .as_ref()
                .map(|c| c.into())
                .or_else(|| mapping.ip_overrides.first().map(|o| (&o.credential).into()));

            let Some(discriminant) = discriminant else {
                continue;
            };

            if cancel.is_cancelled() {
                return Err(Error::msg("Discovery was cancelled"));
            }

            let integration = IntegrationRegistry::get(discriminant);

            // Collect credentials for this IP in specificity order (override → default)
            let credentials: Vec<(
                &crate::server::credentials::r#impl::mapping::CredentialQueryPayload,
                Option<Uuid>,
            )> = {
                let mut creds = Vec::new();
                // IP override first (most specific)
                if let Some(o) = mapping.ip_overrides.iter().find(|o| o.ip == ip) {
                    let cred_id = if o.credential_id != Uuid::nil() {
                        Some(o.credential_id)
                    } else {
                        None
                    };
                    creds.push((&o.credential, cred_id));
                }
                // Network default as fallback
                if let Some(default) = &mapping.default_credential {
                    // Only add if we didn't already have an override
                    if creds.is_empty() {
                        creds.push((default, None));
                    }
                }
                creds
            };

            if credentials.is_empty() {
                continue;
            }

            // Check probe gate ports
            let gate_ports = integration.probe_gate_ports(credentials[0].0);
            if !gate_ports.is_empty() && !gate_ports.iter().all(|gp| open_ports.contains(gp)) {
                continue;
            }

            // Try each credential until probe succeeds
            for (credential, cred_id) in &credentials {
                if cancel.is_cancelled() {
                    return Err(Error::msg("Discovery was cancelled"));
                }

                let probe_ctx = ProbeContext {
                    ip,
                    credential,
                    credential_id: *cred_id,
                    cancel: &cancel,
                    utils: &utils,
                };

                match integration.probe(&probe_ctx).await {
                    Ok(success) => {
                        tracing::info!(
                            ip = %ip,
                            integration = ?discriminant,
                            ports = ?success.ports,
                            "Integration probe succeeded"
                        );
                        // Add probe ports to open_ports
                        for port in &success.ports {
                            if !open_ports.contains(port) {
                                open_ports.push(*port);
                            }
                        }
                        client_responses.insert(success.client_probe, success.ports);
                        if let Some(handle) = success.handle {
                            probe_handles.insert(discriminant, handle);
                        }
                        if let Some(id) = cred_id {
                            working_credential_ids
                                .insert(discriminant, (*id, (*credential).clone()));
                        }
                        // Mark integration probe cost as completed
                        if let Some(counter) = completed_cost {
                            counter.fetch_add(
                                integration.estimated_seconds() as usize * 100,
                                Ordering::Relaxed,
                            );
                        }
                        break;
                    }
                    Err(failure) => {
                        tracing::debug!(
                            ip = %ip,
                            integration = ?discriminant,
                            error = %failure,
                            "Integration probe failed, trying next credential"
                        );
                    }
                }
            }
        }

        // Endpoint scanning
        let mut ports_to_check = open_ports.clone();
        let endpoint_only_ports = Service::endpoint_only_ports();
        ports_to_check.extend(endpoint_only_ports);
        ports_to_check.sort_by_key(|p| (p.number(), p.protocol()));
        ports_to_check.dedup();

        let accept_invalid_certs = ops.config_store.get_accept_invalid_scan_certs().await?;

        let endpoint_responses = scan_endpoints(
            ip,
            cancel.clone(),
            Some(ports_to_check),
            Some(use_https_ports),
            effective_batch_size,
            probe_raw_socket_ports,
            accept_invalid_certs,
        )
        .await?;

        for endpoint_response in &endpoint_responses {
            let port = endpoint_response.endpoint.port_type;
            if !open_ports.contains(&port) {
                open_ports.push(port);
            }
        }

        open_ports.sort_by_key(|p| (p.number(), p.protocol()));
        open_ports.dedup();

        if cancel.is_cancelled() {
            return Err(Error::msg("Discovery was cancelled"));
        }

        tracing::info!(
            ip = %ip,
            open_ports = open_ports.len(),
            endpoints = endpoint_responses.len(),
            "Deep scan complete"
        );

        // DNS hostname lookup (SNMP sysName fallback now handled by SnmpIntegration.execute())
        let hostname = self.get_hostname_for_ip(ip).await?;
        // MAC enrichment from SNMP ipAddrTable now handled by SnmpIntegration.execute()
        let interface = Interface::new(InterfaceBase {
            network_id: subnet.base.network_id,
            host_id: Uuid::nil(), // Placeholder - server will set correct host_id
            name: None,
            subnet_id: subnet.id,
            ip_address: ip,
            mac_address: mac,
            position: 0,
        });

        // Filter raw socket ports from service matching when probe_raw_socket_ports is off,
        // matching the same filtering applied in scan_endpoints() for endpoint probing.
        if !probe_raw_socket_ports {
            open_ports.retain(|p| !p.is_raw_socket());
        }

        if let Ok(Some(mut host_data)) = ops
            .build_host_from_scan(
                ServiceMatchBaselineParams {
                    subnet,
                    interface: &interface,
                    all_ports: &open_ports,
                    endpoint_responses: &endpoint_responses,
                    virtualization: &None,
                    client_responses: &client_responses,
                },
                hostname,
                self.host_naming_fallback,
            )
            .await
        {
            // Reuse the early-reported host ID so the server updates the existing record
            host_data.host.id = early_host_id;

            // --- Integration execute dispatch ---
            // Run execute() for all integrations whose probe succeeded and service matched.

            for mapping in credential_mappings {
                let discriminant: Option<CredentialQueryPayloadDiscriminants> = mapping
                    .default_credential
                    .as_ref()
                    .map(|c| c.into())
                    .or_else(|| mapping.ip_overrides.first().map(|o| (&o.credential).into()));

                let Some(discriminant) = discriminant else {
                    continue;
                };

                let integration = IntegrationRegistry::get(discriminant);

                // Find credential for this IP
                let (credential, cred_id) =
                    if let Some(o) = mapping.ip_overrides.iter().find(|o| o.ip == ip) {
                        (
                            &o.credential,
                            if o.credential_id != Uuid::nil() {
                                Some(o.credential_id)
                            } else {
                                None
                            },
                        )
                    } else if let Some(default) = &mapping.default_credential {
                        (default, None)
                    } else {
                        continue;
                    };

                // Check if integration's associated service was matched
                let cred_type_discriminant: crate::server::credentials::r#impl::types::CredentialTypeDiscriminants = discriminant.into();
                let associated_service = cred_type_discriminant
                    .to_credential_type()
                    .associated_service();
                let service_matched = host_data
                    .services
                    .iter()
                    .any(|s| s.base.service_definition.id() == associated_service.id());

                if !service_matched {
                    continue;
                }

                let accept_invalid_certs = ops
                    .config_store
                    .get_accept_invalid_scan_certs()
                    .await
                    .unwrap_or(false);

                // Clone services to avoid borrow conflict (host_data borrowed
                // both immutably for context and mutably for execute)
                let matched_services_snapshot = host_data.services.clone();

                // Get probe handle for this integration (if probe succeeded)
                let probe_handle_ref = probe_handles
                    .get(&discriminant)
                    .map(|h| h.as_ref() as &(dyn std::any::Any + Send + Sync));

                let ctx = IntegrationContext {
                    ip,
                    credential,
                    credential_id: cred_id,
                    cancel: &cancel,
                    ops: &ops,
                    utils: &utils,
                    probe_handle: probe_handle_ref,
                    matched_services: &matched_services_snapshot,
                    open_ports: &open_ports,
                    endpoint_responses: &endpoint_responses,
                    host_id: early_host_id,
                    host_naming_fallback: self.host_naming_fallback,
                    created_subnets: &[],
                    accept_invalid_certs,
                    scanning_subnet: Some(subnet),
                };

                if let Err(e) = execute_with_progress_reporting(
                    integration.as_ref(),
                    &ctx,
                    &mut host_data,
                    || async {
                        let _ = ops.report_progress(0).await;
                    },
                )
                .await
                {
                    tracing::debug!(
                        ip = %ip,
                        integration = ?discriminant,
                        error = %e,
                        "Integration execute failed"
                    );
                }
            }

            // Populate credential_assignments from successful integration probes
            // whose execute() doesn't handle credential assignments itself.
            // SNMP is handled by SnmpIntegration.execute().
            for (discriminant, (cred_id, _credential)) in &working_credential_ids {
                if *discriminant == CredentialQueryPayloadDiscriminants::Snmp {
                    continue;
                }
                host_data
                    .host
                    .base
                    .credential_assignments
                    .push(CredentialAssignment {
                        credential_id: *cred_id,
                        interface_ids: Some(vec![interface.id]),
                    });
            }

            // Extract final state from host_data
            let host = host_data.host;
            let interfaces = host_data.interfaces;
            let ports = host_data.ports;
            let services = host_data.services;
            let if_entries = host_data.if_entries;

            let services_count = services.len();
            let if_entries_count = if_entries.len();

            if let Ok(host_response) = ops
                .create_host(host, interfaces, ports, services, if_entries, &cancel)
                .await
            {
                tracing::info!(
                    ip = %ip,
                    services = services_count,
                    if_entries = if_entries_count,
                    "Host created"
                );
                let host_data = DiscoveredHostData {
                    docker_service_id: host_response
                        .services
                        .iter()
                        .find(|s| s.base.service_definition.id() == "Docker")
                        .map(|s| s.id),
                    interfaces: host_response.interfaces.clone(),
                };
                return Ok(Some((ip, host_response.to_host(), host_data)));
            } else {
                tracing::warn!(ip = %ip, "Host creation failed");
            }
        } else {
            tracing::debug!(ip = %ip, "Host processing returned None");
        }

        Ok(None)
    }

    async fn get_hostname_for_ip(&self, ip: IpAddr) -> Result<Option<String>, Error> {
        match timeout(Duration::from_millis(2000), async {
            tokio::task::spawn_blocking(move || dns_lookup::lookup_addr(&ip)).await?
        })
        .await
        {
            Ok(Ok(hostname)) => Ok(Some(hostname)),
            _ => Ok(None),
        }
    }

    /// Figure out what order to scan IPs in given allocation patterns
    fn determine_scan_order(&self, subnet: &IpCidr) -> impl Iterator<Item = IpAddr> {
        let mut ips: Vec<IpAddr> = subnet.iter().map(|ip| ip.address()).collect();

        // Sort by likelihood of being active hosts - highest probability first
        ips.sort_by_key(|ip| {
            let last_octet = match ip {
                IpAddr::V4(ipv4) => ipv4.octets()[3],
                IpAddr::V6(_) => return 9999, // IPv6 gets lowest priority for now
            };

            match last_octet {
                // Tier 1: Almost guaranteed to be active infrastructure
                1 => 1,   // Default gateway (.1)
                254 => 2, // Alternative gateway (.254)

                // Tier 2: Very common infrastructure and static assignments
                2 => 10,   // Secondary router/switch
                3 => 11,   // Tertiary infrastructure
                10 => 12,  // Common DHCP start
                100 => 13, // Common DHCP end
                253 => 14, // Alt gateway range
                252 => 15, // Alt gateway range

                // Tier 3: Common static device ranges
                4..=9 => 20 + last_octet as u16, // Infrastructure devices
                11..=20 => 30 + last_octet as u16, // Servers, printers
                21..=30 => 50 + last_octet as u16, // Network devices

                // Tier 4: Active DHCP ranges (most devices live here)
                31..=50 => 100 + last_octet as u16, // Early DHCP range
                51..=100 => 200 + last_octet as u16, // Mid DHCP range
                101..=150 => 400 + last_octet as u16, // Late DHCP range

                // Tier 5: Less common but still viable
                151..=200 => 600 + last_octet as u16, // Extended DHCP
                201..=251 => 800 + last_octet as u16, // High static range

                // Skip entirely - reserved addresses
                0 | 255 => 9998, // Network/broadcast addresses
            }
        });

        ips.into_iter()
    }
}

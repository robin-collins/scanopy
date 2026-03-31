use crate::daemon::discovery::integration::{
    IntegrationContext, IntegrationRegistry, ProbeContext, execute_with_progress_reporting,
};
use crate::daemon::discovery::service::base::DiscoveryRunner;
use crate::daemon::discovery::service::ops::DiscoveryOps;
use crate::daemon::utils::base::{DaemonUtils, merge_host_and_docker_subnets};
use crate::server::bindings::r#impl::base::Binding;
use crate::server::credentials::r#impl::mapping::{
    CredentialMapping, CredentialQueryPayload, CredentialQueryPayloadDiscriminants,
};
use crate::server::daemons::r#impl::api::DaemonDiscoveryRequest;
use crate::server::discovery::r#impl::scan_settings::ScanSettings;
use crate::server::discovery::r#impl::types::{DiscoveryType, HostNamingFallback};
use crate::server::hosts::r#impl::base::{Host, HostBase};
use crate::server::interfaces::r#impl::base::{ALL_INTERFACES_IP, Interface};
use crate::server::ports::r#impl::base::{Port, PortType};
use crate::server::services::definitions::scanopy_daemon::ScanopyDaemon;
use crate::server::services::r#impl::base::{Service, ServiceBase};
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::MatchDetails;
use crate::server::shared::storage::traits::Storable;
use crate::server::shared::types::entities::{DiscoveryMetadata, EntitySource};
use crate::server::subnets::r#impl::base::Subnet;
use anyhow::{Error, Result};
use futures::future::join_all;
use std::net::{IpAddr, Ipv4Addr};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub struct UnifiedDiscovery {
    pub host_id: Uuid,
    pub subnet_ids: Option<Vec<Uuid>>,
    pub host_naming_fallback: HostNamingFallback,
    pub scan_settings: ScanSettings,
    pub credential_mappings: Vec<CredentialMapping<CredentialQueryPayload>>,
}

// Phase 1 (0-5%): Self-report + localhost integrations.
// Phase 2 (5-100%): Network scan with per-host integration probe + execute.

impl DiscoveryRunner {
    pub fn discovery_type(&self) -> DiscoveryType {
        DiscoveryType::Unified {
            host_id: self.params.host_id,
            subnet_ids: self.params.subnet_ids.clone(),
            host_naming_fallback: self.params.host_naming_fallback,
            scan_settings: self.params.scan_settings.clone(),
        }
    }

    pub async fn discover(
        &mut self,
        request: DaemonDiscoveryRequest,
        cancel: CancellationToken,
    ) -> Result<(), Error> {
        let is_first_run = !self.service.config_store.has_self_reported().await;
        let gateway_ips = self
            .service
            .utils
            .get_own_routing_table_gateway_ips()
            .await?;
        let ops = DiscoveryOps::new(&self.service, self.discovery_type());

        // Inject DockerSocket credential if local socket is accessible and enabled
        let enable_local = self
            .service
            .config_store
            .get_enable_local_docker_socket()
            .await
            .unwrap_or(true);
        if enable_local {
            // Check if Docker socket is actually accessible
            let can_connect = self
                .service
                .utils
                .new_docker_client(Ok(None), Ok(None))
                .await
                .is_ok();
            if can_connect {
                // Check if we already have a DockerSocket credential (avoid duplicates)
                let already_has = self.params.credential_mappings.iter().any(|m| {
                    m.default_credential
                        .as_ref()
                        .is_some_and(|c| matches!(c, CredentialQueryPayload::DockerSocket(_)))
                        || m.ip_overrides.iter().any(|o| {
                            matches!(o.credential, CredentialQueryPayload::DockerSocket(_))
                        })
                });
                if !already_has {
                    tracing::info!("Injecting DockerSocket credential for local socket access");
                    self.params.credential_mappings.push(
                        CredentialMapping {
                            default_credential: None,
                            ip_overrides: vec![
                                crate::server::credentials::r#impl::mapping::IpOverride {
                                    ip: std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
                                    credential: CredentialQueryPayload::DockerSocket(
                                        crate::server::credentials::r#impl::mapping::DockerSocketQueryCredential {},
                                    ),
                                    credential_id: Uuid::nil(),
                                },
                            ],
                        },
                    );
                }
            }
        }

        // Always try SNMP "public" community on all hosts.
        // Injected as a broadcast default — user-configured credentials (IP overrides) take priority.
        self.params.credential_mappings.push(CredentialMapping {
            default_credential: Some(CredentialQueryPayload::Snmp(
                crate::server::credentials::r#impl::mapping::SnmpQueryCredential {
                    version: crate::server::credentials::r#impl::mapping::SnmpVersion::V2c,
                    community:
                        crate::server::credentials::r#impl::mapping::ResolvableSecret::Value {
                            value: "public".to_string(),
                        },
                },
            )),
            ip_overrides: vec![],
        });

        tracing::info!(
            is_first_run,
            credential_mappings = self.params.credential_mappings.len(),
            "Unified discovery: self_report=0-5%, network=5-100%",
        );

        // Create subnets before session init (like other runners)
        let created_subnets = match self.discover_create_subnets(&ops, &cancel).await {
            Ok(subnets) => subnets,
            Err(e) => {
                let daemon_id = self.service.config_store.get_id().await?;
                if let Err(init_err) = ops
                    .initialize_session(&request, daemon_id, gateway_ips)
                    .await
                {
                    tracing::error!(
                        "Failed to initialize session for error reporting: {}",
                        init_err
                    );
                    return Err(e);
                }
                ops.finish_session(Err(e), cancel).await?;
                return Ok(());
            }
        };

        // Start session
        ops.start_session(&request, gateway_ips).await?;

        // Run the orchestrated phases
        let discovery_result = self
            .run_unified_phases(&ops, &created_subnets, is_first_run, &cancel)
            .await;

        ops.finish_session(discovery_result, cancel).await?;
        Ok(())
    }

    async fn discover_create_subnets(
        &self,
        ops: &DiscoveryOps,
        cancel: &CancellationToken,
    ) -> Result<Vec<Subnet>, Error> {
        let daemon_id = self.service.config_store.get_id().await?;
        let network_id = self
            .service
            .config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network ID not set"))?;

        let utils = &self.service.utils;

        let interface_filter = self.service.config_store.get_interfaces().await?;
        let (_, subnets, _) = utils
            .get_own_interfaces(
                self.discovery_type(),
                daemon_id,
                network_id,
                &interface_filter,
            )
            .await?;

        // Get docker subnets for merging
        let (docker_proxy, docker_proxy_ssl_info, _ssl_temp_handles, _, _) =
            crate::daemon::discovery::integration::docker::proxy::resolve_docker_proxy(
                &self.params.credential_mappings,
                &self.service.config_store,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::debug!(error = %e, "Failed to resolve Docker proxy for subnet discovery");
                (Ok(None), Ok(None), Vec::new(), None, None)
            });

        let docker_subnets = if let Ok(docker_client) = self
            .service
            .utils
            .new_docker_client(docker_proxy, docker_proxy_ssl_info)
            .await
        {
            self.service
                .utils
                .get_subnets_from_docker_networks(
                    daemon_id,
                    network_id,
                    &docker_client,
                    self.discovery_type(),
                    Uuid::nil(),
                )
                .await
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        // Merge host and Docker subnets — host subnets always win on CIDR overlap
        let merged = merge_host_and_docker_subnets(subnets, docker_subnets);

        // Filter out DockerBridge subnets — those are handled by Docker phase
        let subnets_to_create: Vec<Subnet> = merged
            .into_iter()
            .filter(|s| !s.is_docker_bridge_subnet())
            .collect();

        tracing::info!(
            subnet_count = subnets_to_create.len(),
            cidrs = ?subnets_to_create.iter().map(|s| s.base.cidr.to_string()).collect::<Vec<_>>(),
            "Creating subnets for unified discovery"
        );

        let subnet_futures = subnets_to_create.iter().map(|subnet| async move {
            let cidr = subnet.base.cidr;
            match ops.create_subnet(subnet, cancel).await {
                Ok(created) => {
                    tracing::debug!(cidr = %cidr, subnet_id = %created.id, "Subnet created");
                    Some(created)
                }
                Err(e) => {
                    tracing::warn!(cidr = %cidr, error = %e, "Failed to create subnet");
                    None
                }
            }
        });
        let created_subnets: Vec<Subnet> = join_all(subnet_futures)
            .await
            .into_iter()
            .flatten()
            .collect();

        Ok(created_subnets)
    }

    /// Run all unified discovery phases.
    ///
    /// Phase 1 (0-5%): Self-report + localhost integrations.
    /// Phase 2 (5-100%): Network scan with per-host integration probe + execute.
    async fn run_unified_phases(
        &self,
        ops: &DiscoveryOps,
        created_subnets: &[Subnet],
        is_first_run: bool,
        cancel: &CancellationToken,
    ) -> Result<(), Error> {
        let start = std::time::Instant::now();
        let session = ops.get_session().await?;

        // Phase 1: Daemon Host (0-5%)
        session.set_progress_range(0, 5);

        if is_first_run {
            tracing::info!("Running self-report phase (first run)");

            if let Err(e) = self
                .run_self_report_phase(ops, created_subnets, cancel)
                .await
            {
                tracing::error!(error = %e, "Self-report phase failed, continuing with network phase");
            } else if let Err(e) = self.service.config_store.set_has_self_reported().await {
                tracing::warn!(error = %e, "Failed to persist self-report flag");
            }
        }

        // Run localhost integrations (generic — any integration with localhost credential)
        if let Err(e) = self
            .run_localhost_integrations(ops, created_subnets, cancel)
            .await
        {
            tracing::error!(error = %e, "Localhost integration phase failed, continuing");
        }

        ops.report_progress(100).await?;

        if cancel.is_cancelled() {
            return Err(anyhow::anyhow!("Discovery cancelled"));
        }

        // Phase 2: Network Scan (5-100%)
        session.set_progress_range(5, 100);

        let network_hosts = self.run_network_phase(cancel).await?;

        // Completion banner
        self.log_completion_banner(&network_hosts, start.elapsed());

        Ok(())
    }

    /// Run integrations for localhost credentials (e.g., Docker on daemon host).
    /// Generic — dispatches any integration with a credential targeting 127.0.0.1/::1.
    async fn run_localhost_integrations(
        &self,
        ops: &DiscoveryOps,
        created_subnets: &[Subnet],
        cancel: &CancellationToken,
    ) -> Result<(), Error> {
        for mapping in &self.params.credential_mappings {
            // Find localhost-targeted credentials
            let localhost_overrides: Vec<_> = mapping
                .ip_overrides
                .iter()
                .filter(|o| o.is_localhost())
                .collect();

            if localhost_overrides.is_empty() {
                continue;
            }

            for override_entry in localhost_overrides {
                if cancel.is_cancelled() {
                    return Err(anyhow::anyhow!("Discovery cancelled"));
                }

                let discriminant: CredentialQueryPayloadDiscriminants =
                    (&override_entry.credential).into();
                let integration = IntegrationRegistry::get(discriminant);

                let credential = &override_entry.credential;
                let credential_id = if override_entry.credential_id != Uuid::nil() {
                    Some(override_entry.credential_id)
                } else {
                    None
                };

                // Probe
                let probe_ctx = ProbeContext {
                    ip: override_entry.ip,
                    credential,
                    credential_id,
                    cancel,
                    utils: &self.service.utils,
                };

                let probe_result = match integration.probe(&probe_ctx).await {
                    Ok(success) => success,
                    Err(failure) => {
                        tracing::debug!(
                            ip = %override_entry.ip,
                            error = %failure,
                            "Localhost integration probe failed"
                        );
                        continue;
                    }
                };

                // Build HostData via service matching — same flow as deep_scan_host.
                // The probe's ClientProbe feeds into client_responses so the associated
                // service (e.g., Docker daemon) gets matched automatically.
                let mut client_responses = std::collections::HashMap::new();
                client_responses.insert(probe_result.client_probe, probe_result.ports.clone());

                // Find a subnet + interface for the localhost IP
                let host_ip = self
                    .service
                    .utils
                    .get_own_ip_address()
                    .unwrap_or(override_entry.ip);
                let subnet = created_subnets
                    .iter()
                    .find(|s| s.base.cidr.contains(&host_ip))
                    .or_else(|| created_subnets.first());

                let accept_invalid_certs = self
                    .service
                    .config_store
                    .get_accept_invalid_scan_certs()
                    .await
                    .unwrap_or(false);

                let mut host_data = if let Some(subnet) = subnet {
                    let interface =
                        Interface::new(crate::server::interfaces::r#impl::base::InterfaceBase {
                            network_id: subnet.base.network_id,
                            host_id: Uuid::nil(),
                            name: None,
                            subnet_id: subnet.id,
                            ip_address: host_ip,
                            mac_address: None,
                            position: 0,
                        });

                    let params =
                        crate::server::services::r#impl::base::ServiceMatchBaselineParams {
                            subnet,
                            interface: &interface,
                            all_ports: &probe_result.ports,
                            endpoint_responses: &vec![],
                            virtualization: &None,
                            client_responses: &client_responses,
                        };

                    match ops
                        .build_host_from_scan(params, None, self.params.host_naming_fallback)
                        .await?
                    {
                        Some(hd) => hd,
                        None => {
                            tracing::warn!(
                                ip = %override_entry.ip,
                                "Localhost service matching returned no host"
                            );
                            continue;
                        }
                    }
                } else {
                    tracing::warn!(
                        ip = %override_entry.ip,
                        "No subnet found for localhost integration, skipping"
                    );
                    continue;
                };

                host_data.host.id = self.params.host_id;

                let ctx = IntegrationContext {
                    ip: override_entry.ip,
                    credential,
                    credential_id,
                    cancel,
                    ops,
                    utils: &self.service.utils,
                    probe_handle: probe_result.handle.as_deref(),
                    matched_services: &host_data.services.clone(),
                    open_ports: &probe_result.ports,
                    endpoint_responses: &[],
                    host_id: self.params.host_id,
                    host_naming_fallback: self.params.host_naming_fallback,
                    created_subnets,
                    accept_invalid_certs,
                    scanning_subnet: None,
                };

                tracing::info!(
                    ip = %override_entry.ip,
                    integration = ?discriminant,
                    "Running localhost integration"
                );

                match execute_with_progress_reporting(
                    integration.as_ref(),
                    &ctx,
                    &mut host_data,
                    || async {
                        let _ = ops.report_progress(50).await;
                    },
                )
                .await
                {
                    Ok(()) => {
                        // Persist the enriched host_data to the server
                        tracing::info!(
                            ip = %override_entry.ip,
                            services = host_data.services.len(),
                            interfaces = host_data.interfaces.len(),
                            "Persisting localhost integration results"
                        );
                        if let Err(e) = ops
                            .create_host(
                                host_data.host,
                                host_data.interfaces,
                                host_data.ports,
                                host_data.services,
                                host_data.if_entries,
                                host_data.subnets,
                                cancel,
                            )
                            .await
                        {
                            tracing::error!(
                                ip = %override_entry.ip,
                                error = %e,
                                "Failed to persist localhost integration host"
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            ip = %override_entry.ip,
                            error = %e,
                            "Localhost integration execute failed"
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Self-report phase: detect interfaces, update capabilities, create daemon host
    async fn run_self_report_phase(
        &self,
        ops: &DiscoveryOps,
        created_subnets: &[Subnet],
        cancel: &CancellationToken,
    ) -> Result<(), Error> {
        if cancel.is_cancelled() {
            return Err(anyhow::anyhow!("Discovery cancelled"));
        }

        let daemon_id = self.service.config_store.get_id().await?;
        let network_id = self
            .service
            .config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network ID not set"))?;

        let utils = &self.service.utils;
        let host_id = self.params.host_id;

        let binding_address = self.service.config_store.get_bind_address().await?;
        let binding_ip = IpAddr::V4(binding_address.parse::<Ipv4Addr>()?);

        // Get interfaces
        let interface_filter = self.service.config_store.get_interfaces().await?;
        let (interfaces, _, _) = utils
            .get_own_interfaces(
                self.discovery_type(),
                daemon_id,
                network_id,
                &interface_filter,
            )
            .await?;

        // Capabilities are now updated via process_status on every poll — no separate POST needed.

        if cancel.is_cancelled() {
            return Err(anyhow::anyhow!("Discovery cancelled"));
        }

        // Filter interfaces to those with matching created subnets
        let interfaces: Vec<Interface> = interfaces
            .into_iter()
            .filter_map(|mut i| {
                if let Some(subnet) = created_subnets
                    .iter()
                    .find(|s| s.base.cidr.contains(&i.base.ip_address))
                {
                    i.base.subnet_id = subnet.id;
                    return Some(i);
                }
                None
            })
            .collect();

        let daemon_bound_subnet_ids: Vec<Uuid> = if binding_address == ALL_INTERFACES_IP.to_string()
        {
            created_subnets.iter().map(|s| s.id).collect()
        } else {
            created_subnets
                .iter()
                .filter(|s| s.base.cidr.contains(&binding_ip))
                .map(|s| s.id)
                .collect()
        };

        let own_port = Port::new_hostless(PortType::new_tcp(
            self.service.config_store.get_port().await?,
        ));
        let own_port_id = own_port.id;
        let local_ip = utils.get_own_ip_address()?;
        let hostname = utils.get_own_hostname();

        let host_base = HostBase {
            name: hostname.clone().unwrap_or(format!("{}", local_ip)),
            hostname,
            network_id,
            description: Some("Scanopy daemon".to_string()),
            tags: Vec::new(),
            source: EntitySource::Discovery {
                metadata: vec![DiscoveryMetadata::new(self.discovery_type(), daemon_id)],
            },
            hidden: false,
            virtualization: None,
            sys_descr: None,
            sys_object_id: None,
            sys_location: None,
            sys_contact: None,
            management_url: None,
            chassis_id: None,
            sys_name: None,
            manufacturer: None,
            model: None,
            serial_number: None,
            credential_assignments: vec![],
        };

        let mut host = Host::new(host_base);
        host.id = host_id;

        let daemon_service_definition = ScanopyDaemon;
        let daemon_service_bound_interfaces: Vec<&Interface> = interfaces
            .iter()
            .filter(|i| daemon_bound_subnet_ids.contains(&i.base.subnet_id))
            .collect();

        let daemon_service = Service::new(ServiceBase {
            name: ServiceDefinition::name(&daemon_service_definition).to_string(),
            service_definition: Box::new(daemon_service_definition),
            tags: Vec::new(),
            network_id,
            bindings: daemon_service_bound_interfaces
                .iter()
                .map(|i| Binding::new_port_serviceless(own_port_id, Some(i.id)))
                .collect(),
            host_id: host.id,
            virtualization: None,
            source: EntitySource::DiscoveryWithMatch {
                metadata: vec![DiscoveryMetadata::new(self.discovery_type(), daemon_id)],
                details: MatchDetails::new_certain("Scanopy Daemon self-report"),
            },
            position: 0,
        });

        if cancel.is_cancelled() {
            return Err(anyhow::anyhow!("Discovery cancelled"));
        }

        ops.create_host(
            host,
            interfaces.clone(),
            vec![own_port],
            vec![daemon_service],
            vec![],
            vec![],
            cancel,
        )
        .await?;

        Ok(())
    }

    /// Network phase: run ARP + deep scan to discover hosts and services
    async fn run_network_phase(
        &self,
        cancel: &CancellationToken,
    ) -> Result<Vec<(IpAddr, Host, super::network::DiscoveredHostData)>, Error> {
        // Network discovery owns subnet resolution — unified just coordinates
        let network_discovery = super::network::NetworkScan::new(
            self.params.subnet_ids.clone(),
            self.params.host_naming_fallback,
            self.params.scan_settings.clone(),
            self.params.credential_mappings.clone(),
        );

        let ops = super::ops::DiscoveryOps::new(&self.service, self.discovery_type());
        let utils = &self.service.utils;

        let network_subnets = network_discovery
            .discover_create_subnets(&ops, utils, self.discovery_type(), cancel)
            .await?;

        tracing::info!(
            cidrs = ?network_subnets.iter().map(|s| s.base.cidr.to_string()).collect::<Vec<_>>(),
            "Running network scan phase"
        );

        // scan_and_process_hosts uses the active session
        // (set by our start_discovery call above)
        let network_result = network_discovery
            .scan_and_process_hosts(network_subnets, cancel.clone(), &ops, utils)
            .await;

        match &network_result {
            Ok(hosts) => {
                tracing::info!(
                    hosts_discovered = hosts.len(),
                    "Network scan phase complete"
                );
            }
            Err(_) if cancel.is_cancelled() => {
                return Err(anyhow::anyhow!("Discovery cancelled"));
            }
            Err(e) => {
                tracing::error!(error = %e, "Network scan phase failed");
            }
        }

        network_result
    }

    /// Log a summary banner at the end of discovery, matching the start banner format.
    fn log_completion_banner(
        &self,
        network_hosts: &[(IpAddr, Host, super::network::DiscoveredHostData)],
        duration: std::time::Duration,
    ) {
        let hosts_discovered = network_hosts.len();
        let scan_type = if self.params.scan_settings.is_full_scan {
            "full"
        } else {
            "light"
        };

        // Count credential assignments across discovered hosts
        let mut snmp_mapped = 0u32;
        let mut docker_mapped = 0u32;
        let mut snmp_details: Vec<String> = Vec::new();
        let mut docker_details: Vec<String> = Vec::new();

        for (ip, host, _) in network_hosts {
            for assignment in &host.base.credential_assignments {
                if assignment.interface_ids.is_some() {
                    docker_mapped += 1;
                    docker_details.push(format!("{} → {}", assignment.credential_id, ip));
                } else {
                    snmp_mapped += 1;
                    snmp_details.push(format!("{} → {}", assignment.credential_id, ip));
                }
            }
        }

        let total_credential_mappings = self.params.credential_mappings.len();

        // Banner
        tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        tracing::info!("  Discovery Complete");
        tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        tracing::info!("  {:<20}{}", "Hosts Discovered:", hosts_discovered);
        tracing::info!("  {:<20}{}s", "Duration:", duration.as_secs());
        tracing::info!("  {:<20}{}", "Scan Type:", scan_type);

        if total_credential_mappings > 0 {
            tracing::info!("  ───────────────────────────────────────────────────────────");
            tracing::info!("  Credential Mappings:");
            tracing::info!("    {:<20}{} hosts", "SNMP:", snmp_mapped);
            tracing::info!("    {:<20}{} hosts", "Docker:", docker_mapped);

            for detail in &snmp_details {
                tracing::info!("    SNMP  {}", detail);
            }
            for detail in &docker_details {
                tracing::info!("    Docker  {}", detail);
            }
        }

        tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    }
}

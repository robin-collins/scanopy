use crate::daemon::discovery::service::base::{
    CreatesDiscoveredEntities, DiscoversNetworkedEntities, DiscoveryRunner, RunsDiscovery,
};
use crate::daemon::utils::base::{DaemonUtils, merge_host_and_docker_subnets};
use crate::server::bindings::r#impl::base::Binding;
use crate::server::credentials::r#impl::mapping::{
    CredentialMapping, CredentialQueryPayload, SnmpCredentialMapping,
};
use crate::server::daemons::r#impl::api::{DaemonCapabilities, DaemonDiscoveryRequest};
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
use async_trait::async_trait;
use futures::future::join_all;
use std::net::{IpAddr, Ipv4Addr};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub struct UnifiedDiscovery {
    pub host_id: Uuid,
    pub subnet_ids: Option<Vec<Uuid>>,
    pub scan_local_docker_socket: bool,
    pub host_naming_fallback: HostNamingFallback,
    pub scan_settings: ScanSettings,
    pub credential_mappings: Vec<CredentialMapping<CredentialQueryPayload>>,
}

/// Progress allocation for each phase
struct PhaseAllocations {
    self_report_start: u8,
    self_report_end: u8,
    network_start: u8,
    network_end: u8,
    docker_start: u8,
    docker_end: u8,
}

impl PhaseAllocations {
    fn compute(run_self_report: bool, run_docker: bool) -> Self {
        match (run_self_report, run_docker) {
            // All three phases: 5/80/15
            (true, true) => Self {
                self_report_start: 0,
                self_report_end: 5,
                network_start: 5,
                network_end: 85,
                docker_start: 85,
                docker_end: 100,
            },
            // No self-report: redistribute 5% proportionally to 80:15
            (false, true) => Self {
                self_report_start: 0,
                self_report_end: 0,
                network_start: 0,
                network_end: 84,
                docker_start: 84,
                docker_end: 100,
            },
            // No docker: redistribute 15% proportionally to 5:80
            (true, false) => Self {
                self_report_start: 0,
                self_report_end: 6,
                network_start: 6,
                network_end: 100,
                docker_start: 100,
                docker_end: 100,
            },
            // Network only
            (false, false) => Self {
                self_report_start: 0,
                self_report_end: 0,
                network_start: 0,
                network_end: 100,
                docker_start: 100,
                docker_end: 100,
            },
        }
    }
}

impl CreatesDiscoveredEntities for DiscoveryRunner<UnifiedDiscovery> {}

#[async_trait]
impl DiscoversNetworkedEntities for DiscoveryRunner<UnifiedDiscovery> {
    async fn get_gateway_ips(&self) -> Result<Vec<IpAddr>, Error> {
        self.as_ref()
            .utils
            .get_own_routing_table_gateway_ips()
            .await
    }

    async fn discover_create_subnets(
        &self,
        cancel: &CancellationToken,
    ) -> Result<Vec<Subnet>, Error> {
        let daemon_id = self.as_ref().config_store.get_id().await?;
        let network_id = self
            .as_ref()
            .config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network ID not set"))?;

        let utils = &self.as_ref().utils;

        let interface_filter = self.as_ref().config_store.get_interfaces().await?;
        let (_, subnets, _) = utils
            .get_own_interfaces(
                self.discovery_type(),
                daemon_id,
                network_id,
                &interface_filter,
            )
            .await?;

        // Get docker subnets for merging
        let docker_proxy = self.as_ref().config_store.get_docker_proxy().await;
        let docker_proxy_ssl_info = self.as_ref().config_store.get_docker_proxy_ssl_info().await;

        let docker_subnets = if let Ok(docker_client) = self
            .as_ref()
            .utils
            .new_local_docker_client(docker_proxy, docker_proxy_ssl_info)
            .await
        {
            self.as_ref()
                .utils
                .get_subnets_from_docker_networks(
                    daemon_id,
                    network_id,
                    &docker_client,
                    self.discovery_type(),
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
            match self.create_subnet(subnet, cancel).await {
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
}

#[async_trait]
impl RunsDiscovery for DiscoveryRunner<UnifiedDiscovery> {
    fn discovery_type(&self) -> DiscoveryType {
        DiscoveryType::Unified {
            host_id: self.domain.host_id,
            subnet_ids: self.domain.subnet_ids.clone(),
            scan_local_docker_socket: self.domain.scan_local_docker_socket,
            host_naming_fallback: self.domain.host_naming_fallback,
            scan_settings: self.domain.scan_settings.clone(),
        }
    }

    async fn discover(
        &self,
        request: DaemonDiscoveryRequest,
        cancel: CancellationToken,
    ) -> Result<(), Error> {
        // Log credential mappings
        self.log_credential_mappings();

        // Determine which phases to run
        let is_first_run = self
            .as_ref()
            .config_store
            .get_capabilities()
            .await
            .map(|c| c.interfaced_subnet_ids.is_empty())
            .unwrap_or(true);

        let run_docker = self.should_run_docker_phase();

        let alloc = PhaseAllocations::compute(is_first_run, run_docker);

        tracing::info!(
            is_first_run,
            run_docker,
            "Unified discovery phase plan: self_report={}-{}%, network={}-{}%, docker={}-{}%",
            alloc.self_report_start,
            alloc.self_report_end,
            alloc.network_start,
            alloc.network_end,
            alloc.docker_start,
            alloc.docker_end,
        );

        // Create subnets before session init (like other runners)
        let created_subnets = match self.discover_create_subnets(&cancel).await {
            Ok(subnets) => subnets,
            Err(e) => {
                let daemon_id = self.as_ref().config_store.get_id().await?;
                if let Err(init_err) = self.initialize_discovery_session(request, daemon_id).await {
                    tracing::error!(
                        "Failed to initialize session for error reporting: {}",
                        init_err
                    );
                    return Err(e);
                }
                self.finish_discovery(Err(e), cancel).await?;
                return Ok(());
            }
        };

        // Start session
        self.start_discovery(request).await?;

        // Run the orchestrated phases
        let discovery_result = self
            .run_unified_phases(&created_subnets, &alloc, is_first_run, run_docker, &cancel)
            .await;

        self.finish_discovery(discovery_result, cancel).await?;
        Ok(())
    }
}

impl DiscoveryRunner<UnifiedDiscovery> {
    /// Log credential mappings at discovery start
    fn log_credential_mappings(&self) {
        for mapping in &self.domain.credential_mappings {
            if let Some(ref default) = mapping.default_credential {
                tracing::info!(
                    "Using ******** for {} on all scanned hosts",
                    default.discovery_label()
                );
            }

            for ip_override in &mapping.ip_overrides {
                tracing::info!(
                    "Using ******** for {} on {} (host override)",
                    ip_override.credential.discovery_label(),
                    ip_override.ip,
                );
            }
        }
    }

    /// Check if docker phase should run
    fn should_run_docker_phase(&self) -> bool {
        if self.domain.scan_local_docker_socket {
            return true;
        }
        // Check if any credential mapping has a DockerProxy credential
        self.domain.credential_mappings.iter().any(|m| {
            m.default_credential
                .as_ref()
                .is_some_and(|c| matches!(c, CredentialQueryPayload::DockerProxy(_)))
                || m.ip_overrides
                    .iter()
                    .any(|o| matches!(o.credential, CredentialQueryPayload::DockerProxy(_)))
        })
    }

    /// Extract SNMP credentials from credential_mappings into SnmpCredentialMapping
    fn extract_snmp_credential_mapping(&self) -> SnmpCredentialMapping {
        for mapping in &self.domain.credential_mappings {
            let default_credential = mapping.default_credential.as_ref().and_then(|c| match c {
                CredentialQueryPayload::Snmp(snmp) => Some(snmp.clone()),
                _ => None,
            });

            let ip_overrides: Vec<_> = mapping
                .ip_overrides
                .iter()
                .filter_map(|o| match &o.credential {
                    CredentialQueryPayload::Snmp(snmp) => {
                        Some(crate::server::credentials::r#impl::mapping::IpOverride {
                            ip: o.ip,
                            credential: snmp.clone(),
                        })
                    }
                    _ => None,
                })
                .collect();

            if default_credential.is_some() || !ip_overrides.is_empty() {
                return SnmpCredentialMapping {
                    default_credential,
                    ip_overrides,
                    required_ports: mapping.required_ports.clone(),
                };
            }
        }

        SnmpCredentialMapping::default()
    }

    /// Resolve Docker proxy config from credential_mappings, falling back to AppConfig
    async fn resolve_docker_proxy(
        &self,
    ) -> Result<(
        Result<Option<String>, Error>,
        Result<Option<(String, String, String)>, Error>,
    )> {
        // Check credential_mappings for DockerProxy
        for mapping in &self.domain.credential_mappings {
            let docker_cred = mapping
                .default_credential
                .as_ref()
                .and_then(|c| match c {
                    CredentialQueryPayload::DockerProxy(d) => Some(d),
                    _ => None,
                })
                .or_else(|| {
                    mapping
                        .ip_overrides
                        .iter()
                        .find_map(|o| match &o.credential {
                            CredentialQueryPayload::DockerProxy(d) => Some(d),
                            _ => None,
                        })
                });

            if let Some(docker_cred) = docker_cred {
                let label = "Docker proxy connection";

                // Build proxy URL
                let proxy_path = docker_cred
                    .path
                    .as_deref()
                    .unwrap_or("")
                    .trim_start_matches('/');
                let has_ssl = docker_cred.ssl_cert.is_some();
                let scheme = if has_ssl { "https" } else { "http" };
                let proxy_url = if proxy_path.is_empty() {
                    format!("{}://localhost:{}", scheme, docker_cred.port)
                } else {
                    format!("{}://localhost:{}/{}", scheme, docker_cred.port, proxy_path)
                };

                // Resolve SSL if present
                let ssl_info = if let (Some(cert_rv), Some(key_rv), Some(chain_rv)) = (
                    &docker_cred.ssl_cert,
                    &docker_cred.ssl_key,
                    &docker_cred.ssl_chain,
                ) {
                    let cert = cert_rv.resolve("ssl_cert", label)?;
                    let key_secret = key_rv.resolve("ssl_key", label)?;
                    let chain = chain_rv.resolve("ssl_chain", label)?;
                    Ok(Some((cert, key_secret.expose_secret().clone(), chain)))
                } else {
                    Ok(None)
                };

                return Ok((Ok(Some(proxy_url)), ssl_info));
            }
        }

        // Fall back to AppConfig with deprecation warning
        let docker_proxy = self.as_ref().config_store.get_docker_proxy().await;
        let docker_proxy_ssl_info = self.as_ref().config_store.get_docker_proxy_ssl_info().await;

        Ok((docker_proxy, docker_proxy_ssl_info))
    }

    /// Run all unified discovery phases
    async fn run_unified_phases(
        &self,
        created_subnets: &[Subnet],
        alloc: &PhaseAllocations,
        is_first_run: bool,
        run_docker: bool,
        cancel: &CancellationToken,
    ) -> Result<(), Error> {
        // Phase 1: Self-report (first run only)
        if is_first_run {
            tracing::info!("Running self-report phase (first run)");
            self.report_scanning_progress(alloc.self_report_start)
                .await?;

            if let Err(e) = self.run_self_report_phase(created_subnets, cancel).await {
                tracing::error!(error = %e, "Self-report phase failed, continuing with network phase");
            }

            self.report_scanning_progress(alloc.self_report_end).await?;
        }

        if cancel.is_cancelled() {
            return Err(anyhow::anyhow!("Discovery cancelled"));
        }

        // Phase 2: Network scan
        tracing::info!("Running network scan phase");
        self.report_scanning_progress(alloc.network_start).await?;

        // Use the network runner's scan_and_process_hosts via a temporary NetworkScanDiscovery
        let snmp_credentials = self.extract_snmp_credential_mapping();
        let network_discovery = super::network::NetworkScanDiscovery::new(
            self.domain.subnet_ids.clone(),
            self.domain.host_naming_fallback,
            snmp_credentials,
            self.domain.scan_settings.clone(),
        );

        let network_runner = DiscoveryRunner::new(
            self.service.clone(),
            self.manager.clone(),
            network_discovery,
        );

        // The network runner's scan_and_process_hosts uses the active session
        // (set by our start_discovery call above)
        let network_result = network_runner
            .scan_and_process_hosts(created_subnets.to_vec(), cancel.clone())
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

        self.report_scanning_progress(alloc.network_end).await?;

        if cancel.is_cancelled() {
            return Err(anyhow::anyhow!("Discovery cancelled"));
        }

        // Phase 3: Docker scan (if applicable)
        if run_docker {
            tracing::info!("Running Docker scan phase");
            self.report_scanning_progress(alloc.docker_start).await?;

            if let Err(e) = self.run_docker_phase(created_subnets, cancel).await {
                tracing::error!(error = %e, "Docker scan phase failed, continuing");
            }

            self.report_scanning_progress(alloc.docker_end).await?;
        }

        // Return error only if network phase failed fatally
        network_result.map(|_| ())
    }

    /// Self-report phase: detect interfaces, update capabilities, create daemon host
    async fn run_self_report_phase(
        &self,
        created_subnets: &[Subnet],
        cancel: &CancellationToken,
    ) -> Result<(), Error> {
        if cancel.is_cancelled() {
            return Err(anyhow::anyhow!("Discovery cancelled"));
        }

        let daemon_id = self.as_ref().config_store.get_id().await?;
        let network_id = self
            .as_ref()
            .config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network ID not set"))?;

        let utils = &self.as_ref().utils;
        let host_id = self.domain.host_id;

        let binding_address = self.as_ref().config_store.get_bind_address().await?;
        let binding_ip = IpAddr::V4(binding_address.parse::<Ipv4Addr>()?);

        // Get interfaces
        let interface_filter = self.as_ref().config_store.get_interfaces().await?;
        let (interfaces, _, _) = utils
            .get_own_interfaces(
                self.discovery_type(),
                daemon_id,
                network_id,
                &interface_filter,
            )
            .await?;

        // Check Docker availability
        let (docker_proxy, docker_proxy_ssl_info) = self.resolve_docker_proxy().await?;
        let has_docker_socket = self
            .as_ref()
            .utils
            .new_local_docker_client(docker_proxy, docker_proxy_ssl_info)
            .await
            .is_ok();

        // Update capabilities
        let interfaced_subnet_ids: Vec<Uuid> = created_subnets.iter().map(|s| s.id).collect();
        tracing::debug!(
            has_docker_socket,
            subnet_count = interfaced_subnet_ids.len(),
            "Updating capabilities"
        );
        self.update_capabilities(has_docker_socket, interfaced_subnet_ids)
            .await?;

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
            self.as_ref().config_store.get_port().await?,
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

        self.create_host(
            host,
            interfaces.clone(),
            vec![own_port],
            vec![daemon_service],
            vec![],
            cancel,
        )
        .await?;

        Ok(())
    }

    /// Docker phase: resolve proxy, create client, scan containers
    async fn run_docker_phase(
        &self,
        created_subnets: &[Subnet],
        cancel: &CancellationToken,
    ) -> Result<(), Error> {
        if cancel.is_cancelled() {
            return Err(anyhow::anyhow!("Discovery cancelled"));
        }

        let daemon_id = self.as_ref().config_store.get_id().await?;
        let network_id = self
            .as_ref()
            .config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network ID not set"))?;

        // Resolve Docker proxy config
        let (docker_proxy, docker_proxy_ssl_info) = self.resolve_docker_proxy().await?;

        let docker_client = match self
            .as_ref()
            .utils
            .new_local_docker_client(docker_proxy, docker_proxy_ssl_info)
            .await
        {
            Ok(client) => client,
            Err(e) => {
                tracing::warn!(error = %e, "Docker client unavailable, skipping Docker phase");
                return Ok(());
            }
        };

        // Create Docker scan runner using existing DockerScanDiscovery
        let docker_discovery = super::docker::DockerScanDiscovery::new(
            self.domain.host_id,
            self.domain.host_naming_fallback,
        );
        let docker_runner =
            DiscoveryRunner::new(self.service.clone(), self.manager.clone(), docker_discovery);

        // Set docker client on the domain
        docker_runner
            .domain
            .docker_client
            .set(docker_client.clone())
            .map_err(|_| anyhow::anyhow!("Failed to set docker client"))?;

        // Get host interfaces for docker daemon service
        let interface_filter = self.as_ref().config_store.get_interfaces().await?;
        let (mut host_interfaces, _, _) = self
            .as_ref()
            .utils
            .get_own_interfaces(
                self.discovery_type(),
                daemon_id,
                network_id,
                &interface_filter,
            )
            .await?;

        // Update interface subnet IDs to match created subnets
        for interface in &mut host_interfaces {
            if let Some(subnet) = created_subnets
                .iter()
                .find(|s| s.base.cidr.contains(&interface.base.ip_address))
            {
                interface.base.subnet_id = subnet.id;
            }
        }

        // Create Docker daemon service
        let (_, services) = docker_runner
            .create_docker_daemon_service(&host_interfaces, cancel)
            .await?;

        let docker_daemon_service = services
            .first()
            .ok_or_else(|| anyhow::anyhow!("Docker daemon service was not created"))?;

        // Get container info
        let containers = docker_runner.get_containers_and_summaries().await?;

        // Build container interfaces map
        let containers_interfaces_and_subnets = docker_runner.get_container_interfaces(
            &containers,
            created_subnets,
            &mut host_interfaces,
        );

        let result = docker_runner
            .scan_and_process_containers(
                cancel.clone(),
                containers,
                &containers_interfaces_and_subnets,
                &docker_daemon_service.id,
            )
            .await;

        match &result {
            Ok(container_data) => {
                tracing::info!(
                    discovered = container_data.len(),
                    "Docker scan phase complete"
                );
            }
            Err(e) => {
                tracing::error!(error = %e, "Docker container scanning failed");
            }
        }

        result.map(|_| ())
    }

    async fn update_capabilities(
        &self,
        has_docker_socket: bool,
        interfaced_subnet_ids: Vec<Uuid>,
    ) -> Result<(), Error> {
        let capabilities = DaemonCapabilities {
            has_docker_socket,
            interfaced_subnet_ids: interfaced_subnet_ids.clone(),
        };

        self.as_ref()
            .config_store
            .set_capabilities(capabilities.clone())
            .await?;

        let daemon_id = self.as_ref().api_client.config().get_id().await?;
        let path = format!("/api/daemons/{}/update-capabilities", daemon_id);

        match self
            .as_ref()
            .api_client
            .post_no_data(&path, &capabilities, "Failed to update capabilities")
            .await
        {
            Ok(()) => {
                tracing::info!(
                    has_docker_socket,
                    subnet_count = interfaced_subnet_ids.len(),
                    "Daemon capabilities updated successfully"
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to update daemon capabilities");
                Err(e)
            }
        }
    }
}

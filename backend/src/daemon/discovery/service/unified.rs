use crate::daemon::discovery::service::base::{
    CreatesDiscoveredEntities, DiscoversNetworkedEntities, DiscoveryRunner, RunsDiscovery,
};
use crate::daemon::utils::base::{DaemonUtils, merge_host_and_docker_subnets};
use crate::server::bindings::r#impl::base::Binding;
use crate::server::credentials::r#impl::mapping::{
    CredentialMapping, CredentialQueryPayload, SnmpCredentialMapping, SnmpQueryCredential,
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

/// Progress allocation for each phase (order: self-report → docker → network)
struct PhaseAllocations {
    self_report_start: u8,
    self_report_end: u8,
    docker_start: u8,
    docker_end: u8,
    network_start: u8,
    network_end: u8,
}

impl PhaseAllocations {
    fn compute(run_self_report: bool, run_docker: bool) -> Self {
        match (run_self_report, run_docker) {
            // All three phases: 5/15/80 (self-report/docker/network)
            (true, true) => Self {
                self_report_start: 0,
                self_report_end: 5,
                docker_start: 5,
                docker_end: 20,
                network_start: 20,
                network_end: 100,
            },
            // No self-report + docker: 16/84 (docker/network)
            (false, true) => Self {
                self_report_start: 0,
                self_report_end: 0,
                docker_start: 0,
                docker_end: 16,
                network_start: 16,
                network_end: 100,
            },
            // Self-report + no docker: 6/94 (self-report/network)
            (true, false) => Self {
                self_report_start: 0,
                self_report_end: 6,
                docker_start: 0,
                docker_end: 0,
                network_start: 6,
                network_end: 100,
            },
            // Network only
            (false, false) => Self {
                self_report_start: 0,
                self_report_end: 0,
                docker_start: 0,
                docker_end: 0,
                network_start: 0,
                network_end: 100,
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
        let (docker_proxy, docker_proxy_ssl_info, _ssl_temp_handles, _) =
            self.resolve_docker_proxy().await.unwrap_or_else(|e| {
                tracing::debug!(error = %e, "Failed to resolve Docker proxy for subnet discovery");
                (Ok(None), Ok(None), Vec::new(), None)
            });

        let docker_subnets = if let Ok(docker_client) = self
            .as_ref()
            .utils
            .new_docker_client(docker_proxy, docker_proxy_ssl_info)
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
            "Unified discovery phase plan: self_report={}-{}%, docker={}-{}%, network={}-{}%",
            alloc.self_report_start,
            alloc.self_report_end,
            alloc.docker_start,
            alloc.docker_end,
            alloc.network_start,
            alloc.network_end,
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

    /// Extract SNMP credentials from credential_mappings into SnmpCredentialMapping.
    /// Resolves FilePath fields to Value so downstream code doesn't need file I/O.
    /// Caches resolution results per credential to avoid duplicate error logging.
    fn extract_snmp_credential_mapping(&self) -> SnmpCredentialMapping {
        use std::collections::HashMap;

        for mapping in &self.domain.credential_mappings {
            // Cache resolved credentials to avoid duplicate resolution and error logging.
            // All IP overrides sharing the same credential definition will resolve identically,
            // so we resolve once and reuse the result.
            let mut resolve_cache: HashMap<CredentialQueryPayload, Option<SnmpQueryCredential>> =
                HashMap::new();

            let default_credential = mapping.default_credential.as_ref().and_then(|c| {
                let result = match c.resolve_file_paths() {
                    Ok(CredentialQueryPayload::Snmp(snmp)) => Some(snmp),
                    Ok(_) => None,
                    Err(e) => {
                        tracing::error!(error = %e, "Failed to resolve SNMP credential file paths");
                        None
                    }
                };
                resolve_cache.insert(c.clone(), result.clone());
                result
            });

            let ip_overrides: Vec<_> = mapping
                .ip_overrides
                .iter()
                .filter_map(|o| {
                    let resolved = if let Some(cached) = resolve_cache.get(&o.credential) {
                        cached.clone()
                    } else {
                        let result = match o.credential.resolve_file_paths() {
                            Ok(CredentialQueryPayload::Snmp(snmp)) => Some(snmp),
                            Ok(_) => None,
                            Err(e) => {
                                tracing::error!(
                                    error = %e,
                                    "Failed to resolve SNMP credential file paths"
                                );
                                None
                            }
                        };
                        resolve_cache.insert(o.credential.clone(), result.clone());
                        result
                    };
                    resolved.map(
                        |snmp| crate::server::credentials::r#impl::mapping::IpOverride {
                            ip: o.ip,
                            credential: snmp,
                            credential_id: o.credential_id,
                        },
                    )
                })
                .collect();

            if default_credential.is_some() || !ip_overrides.is_empty() {
                return SnmpCredentialMapping {
                    default_credential,
                    ip_overrides,
                };
            }
        }

        SnmpCredentialMapping::default()
    }

    /// Resolve Docker proxy config from credential_mappings, falling back to AppConfig.
    /// Returns (proxy_url, ssl_paths, temp_handles, credential_id).
    /// credential_id is returned for future remote Docker scanning auto-assignment.
    async fn resolve_docker_proxy(
        &self,
    ) -> Result<(
        Result<Option<String>, Error>,
        Result<Option<(String, String, String)>, Error>,
        Vec<tempfile::NamedTempFile>,
        Option<Uuid>,
    )> {
        // Check credential_mappings for DockerProxy targeting localhost only.
        // Remote Docker credentials are handled in deep_scan_host() during network scanning.
        for mapping in &self.domain.credential_mappings {
            let docker_match = mapping.ip_overrides.iter().find(|o| {
                o.is_localhost() && matches!(o.credential, CredentialQueryPayload::DockerProxy(_))
            });

            let (docker_cred, cred_id, override_ip) = if let Some(override_entry) = docker_match {
                let cred = match &override_entry.credential {
                    CredentialQueryPayload::DockerProxy(d) => d,
                    _ => unreachable!(),
                };
                let id = override_entry.credential_id;
                (
                    Some(cred),
                    if id != Uuid::nil() { Some(id) } else { None },
                    Some(override_entry.ip),
                )
            } else if let Some(CredentialQueryPayload::DockerProxy(d)) =
                mapping.default_credential.as_ref()
            {
                (Some(d), None, None) // network-level default, no override IP
            } else {
                (None, None, None)
            };

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
                let host = match override_ip {
                    Some(IpAddr::V6(v6)) => format!("[{}]", v6),
                    Some(ip) => ip.to_string(),
                    None => "127.0.0.1".to_string(),
                };
                let proxy_url = if proxy_path.is_empty() {
                    format!("{}://{}:{}", scheme, host, docker_cred.port)
                } else {
                    format!("{}://{}:{}/{}", scheme, host, docker_cred.port, proxy_path)
                };

                // Resolve SSL to filesystem paths (inline values get written to temp files)
                let mut temp_handles = Vec::new();
                let ssl_info = if let (Some(cert_rv), Some(key_rv), Some(chain_rv)) = (
                    &docker_cred.ssl_cert,
                    &docker_cred.ssl_key,
                    &docker_cred.ssl_chain,
                ) {
                    let (cert_path, cert_handle) = cert_rv.resolve_to_path("ssl_cert", label)?;
                    let (key_path, key_handle) = key_rv.resolve_to_path("ssl_key", label)?;
                    let (chain_path, chain_handle) =
                        chain_rv.resolve_to_path("ssl_chain", label)?;
                    temp_handles.extend(cert_handle);
                    temp_handles.extend(key_handle);
                    temp_handles.extend(chain_handle);
                    Ok(Some((
                        cert_path.to_string_lossy().into_owned(),
                        key_path.to_string_lossy().into_owned(),
                        chain_path.to_string_lossy().into_owned(),
                    )))
                } else {
                    Ok(None)
                };

                tracing::info!(
                    proxy_url = %proxy_url,
                    has_ssl = has_ssl,
                    credential_id = ?cred_id,
                    "Resolved Docker proxy from credential"
                );

                return Ok((Ok(Some(proxy_url)), ssl_info, temp_handles, cred_id));
            }
        }

        // Fall back to AppConfig with deprecation warning (no credential_id)
        tracing::debug!("No Docker proxy credential in mappings, falling back to AppConfig");
        let docker_proxy = self.as_ref().config_store.get_docker_proxy().await;
        let docker_proxy_ssl_info = self.as_ref().config_store.get_docker_proxy_ssl_info().await;

        Ok((docker_proxy, docker_proxy_ssl_info, Vec::new(), None))
    }

    /// Extract all Docker credentials indexed by target IP.
    /// Returns credentials for all IPs that have DockerProxy mappings (both overrides and defaults).
    fn resolve_docker_credentials(
        &self,
    ) -> std::collections::HashMap<
        IpAddr,
        crate::server::credentials::r#impl::mapping::DockerProxyQueryCredential,
    > {
        let mut result = std::collections::HashMap::new();

        for mapping in &self.domain.credential_mappings {
            for override_entry in &mapping.ip_overrides {
                if let CredentialQueryPayload::DockerProxy(_) = &override_entry.credential {
                    match override_entry.credential.resolve_file_paths() {
                        Ok(CredentialQueryPayload::DockerProxy(resolved)) => {
                            result.insert(override_entry.ip, resolved);
                        }
                        Ok(_) => {}
                        Err(e) => {
                            tracing::error!(error = %e, ip = %override_entry.ip, "Failed to resolve Docker credential file paths");
                        }
                    }
                }
            }
        }

        result
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
        let session = self.as_ref().get_session().await?;

        // Phase 1: Self-report + localhost Docker (first run only)
        if is_first_run {
            tracing::info!("Running self-report phase (first run)");
            session.set_progress_range(alloc.self_report_start, alloc.self_report_end);

            if let Err(e) = self.run_self_report_phase(created_subnets, cancel).await {
                tracing::error!(error = %e, "Self-report phase failed, continuing with network phase");
            }

            self.report_scanning_progress(100).await?;
        }

        if cancel.is_cancelled() {
            return Err(anyhow::anyhow!("Discovery cancelled"));
        }

        // Phase 2: Localhost Docker scan (replaces old dedicated Docker phase)
        if run_docker {
            tracing::info!("Running localhost Docker scan phase");
            session.set_progress_range(alloc.docker_start, alloc.docker_end);

            if let Err(e) = self.run_docker_phase(created_subnets, cancel).await {
                tracing::error!(error = %e, "Docker scan phase failed, continuing");
            }

            self.report_scanning_progress(100).await?;
        }

        if cancel.is_cancelled() {
            return Err(anyhow::anyhow!("Discovery cancelled"));
        }

        // Phase 3: Network scan (slow — ARP + deep scan)
        session.set_progress_range(alloc.network_start, alloc.network_end);

        let network_hosts = self.run_network_phase(cancel).await?;

        if cancel.is_cancelled() {
            return Err(anyhow::anyhow!("Discovery cancelled"));
        }

        // Phase 4: Remote Docker container scanning
        // For each host discovered with a Docker service during network scanning,
        // scan its containers if we have Docker credentials for that IP
        if let Err(e) = self.run_remote_docker_phases(&network_hosts, cancel).await {
            tracing::error!(error = %e, "Remote Docker scan phase failed, continuing");
        }

        Ok(())
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
        // _ssl_temp_handles must stay alive until docker_client is dropped — bollard reads
        // key/cert lazily during TLS handshake, so the temp files must exist on disk.
        let (docker_proxy, docker_proxy_ssl_info, _ssl_temp_handles, _docker_cred_id) =
            self.resolve_docker_proxy().await?;

        let docker_client = match self
            .as_ref()
            .utils
            .new_docker_client(docker_proxy, docker_proxy_ssl_info)
            .await
        {
            Ok(client) => client,
            Err(e) => {
                tracing::warn!(error = %e, "Docker client unavailable, skipping Docker phase");
                return Ok(());
            }
        };

        // Create Docker scan runner using existing DockerScanDiscovery
        let docker_discovery = super::docker::DockerScanDiscovery::new_deferred(
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

        // Fetch Docker bridge subnets (filtered out by discover_create_subnets)
        let docker_subnets = self
            .as_ref()
            .utils
            .get_subnets_from_docker_networks(
                daemon_id,
                network_id,
                &docker_client,
                self.discovery_type(),
            )
            .await
            .unwrap_or_default();

        // Create Docker bridge subnets on the server
        let bridge_subnet_futures = docker_subnets
            .iter()
            .filter(|s| s.is_docker_bridge_subnet())
            .map(|s| docker_runner.create_subnet(s, cancel));
        let created_docker_subnets: Vec<Subnet> = join_all(bridge_subnet_futures)
            .await
            .into_iter()
            .filter_map(|r| r.ok())
            .collect();

        // Merge physical + Docker bridge subnets for interface resolution
        let all_subnets: Vec<Subnet> = created_subnets
            .iter()
            .cloned()
            .chain(created_docker_subnets.iter().cloned())
            .collect();

        // Update interface subnet IDs to match all subnets
        for interface in &mut host_interfaces {
            if let Some(subnet) = all_subnets
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

        // Set docker_service_id on domain for scan_and_process_containers
        docker_runner
            .domain
            .docker_service_id
            .set(docker_daemon_service.id)
            .map_err(|_| anyhow::anyhow!("Docker service ID already set"))?;

        // Get container info
        let containers = docker_runner.get_containers_and_summaries().await?;

        // Build container interfaces map using all subnets (including Docker bridge)
        let containers_interfaces_and_subnets =
            docker_runner.get_container_interfaces(&containers, &all_subnets, &mut host_interfaces);

        let result = docker_runner
            .scan_and_process_containers(
                cancel.clone(),
                containers,
                &containers_interfaces_and_subnets,
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

    /// Network phase: run ARP + deep scan to discover hosts and services
    async fn run_network_phase(
        &self,
        cancel: &CancellationToken,
    ) -> Result<Vec<(IpAddr, Host, super::network::DiscoveredServiceIds)>, Error> {
        // Network runner owns subnet resolution — unified just coordinates
        let snmp_credentials = self.extract_snmp_credential_mapping();
        let docker_credentials = self.resolve_docker_credentials();
        let network_discovery = super::network::NetworkScanDiscovery::new(
            self.domain.subnet_ids.clone(),
            self.domain.host_naming_fallback,
            snmp_credentials,
            self.domain.scan_settings.clone(),
            docker_credentials,
        );

        let network_runner = DiscoveryRunner::new(
            self.service.clone(),
            self.manager.clone(),
            network_discovery,
        );

        let network_subnets = network_runner.discover_create_subnets(cancel).await?;

        tracing::info!(
            cidrs = ?network_subnets.iter().map(|s| s.base.cidr.to_string()).collect::<Vec<_>>(),
            "Running network scan phase"
        );

        // The network runner's scan_and_process_hosts uses the active session
        // (set by our start_discovery call above)
        let network_result = network_runner
            .scan_and_process_hosts(network_subnets, cancel.clone())
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

    /// Remote Docker phase: scan containers on remote hosts that have Docker credentials
    async fn run_remote_docker_phases(
        &self,
        network_hosts: &[(IpAddr, Host, super::network::DiscoveredServiceIds)],
        cancel: &CancellationToken,
    ) -> Result<(), Error> {
        let docker_credentials = self.resolve_docker_credentials();
        if docker_credentials.is_empty() {
            return Ok(());
        }

        let mut remote_count = 0u32;

        for (cred_ip, docker_cred) in &docker_credentials {
            if cancel.is_cancelled() {
                return Err(anyhow::anyhow!("Discovery cancelled"));
            }

            // Skip localhost IPs — handled by run_docker_phase
            if cred_ip.is_loopback() {
                continue;
            }

            // Find the matching host from network scan results by scanned IP
            let host_entry = network_hosts
                .iter()
                .find(|(scanned_ip, _, _)| scanned_ip == cred_ip);

            let Some((_, host, service_ids)) = host_entry else {
                tracing::debug!(
                    ip = %cred_ip,
                    "No matching host found for remote Docker credential, skipping"
                );
                continue;
            };

            let Some(docker_service_id) = service_ids.docker else {
                tracing::debug!(
                    ip = %cred_ip,
                    host_id = %host.id,
                    "Host has no Docker service, skipping remote Docker scan"
                );
                continue;
            };

            tracing::info!(
                ip = %cred_ip,
                host_id = %host.id,
                docker_service_id = %docker_service_id,
                "Starting remote Docker container scan"
            );

            // Build proxy URL from credential
            let proxy_path = docker_cred
                .path
                .as_deref()
                .unwrap_or("")
                .trim_start_matches('/');
            let has_ssl = docker_cred.ssl_cert.is_some();
            let scheme = if has_ssl { "https" } else { "http" };
            let host_str = match cred_ip {
                IpAddr::V6(v6) => format!("[{}]", v6),
                _ => cred_ip.to_string(),
            };
            let proxy_url = if proxy_path.is_empty() {
                format!("{}://{}:{}", scheme, host_str, docker_cred.port)
            } else {
                format!(
                    "{}://{}:{}/{}",
                    scheme, host_str, docker_cred.port, proxy_path
                )
            };

            // Resolve SSL paths
            let label = "Remote Docker proxy connection";
            let mut _ssl_temp_handles: Vec<tempfile::NamedTempFile> = Vec::new();
            let ssl_info = if let (Some(cert_rv), Some(key_rv), Some(chain_rv)) = (
                &docker_cred.ssl_cert,
                &docker_cred.ssl_key,
                &docker_cred.ssl_chain,
            ) {
                let (cert_path, cert_handle) = cert_rv.resolve_to_path("ssl_cert", label)?;
                let (key_path, key_handle) = key_rv.resolve_to_path("ssl_key", label)?;
                let (chain_path, chain_handle) = chain_rv.resolve_to_path("ssl_chain", label)?;
                _ssl_temp_handles.extend(cert_handle);
                _ssl_temp_handles.extend(key_handle);
                _ssl_temp_handles.extend(chain_handle);
                Ok(Some((
                    cert_path.to_string_lossy().into_owned(),
                    key_path.to_string_lossy().into_owned(),
                    chain_path.to_string_lossy().into_owned(),
                )))
            } else {
                Ok(None)
            };

            // Connect Docker client
            let docker_client = match self
                .as_ref()
                .utils
                .new_docker_client(Ok(Some(proxy_url.clone())), ssl_info)
                .await
            {
                Ok(client) => client,
                Err(e) => {
                    tracing::warn!(
                        ip = %cred_ip,
                        error = %e,
                        "Failed to connect Docker client for remote scan, skipping"
                    );
                    continue;
                }
            };

            // Create DockerScanDiscovery with host_id and docker_service_id
            let docker_discovery = super::docker::DockerScanDiscovery::new(
                host.id,
                docker_service_id,
                self.domain.host_naming_fallback,
            );
            let docker_runner =
                DiscoveryRunner::new(self.service.clone(), self.manager.clone(), docker_discovery);

            // Set docker client on the domain
            docker_runner
                .domain
                .docker_client
                .set(docker_client)
                .map_err(|_| anyhow::anyhow!("Failed to set docker client"))?;

            // Scan containers
            let containers = match docker_runner.get_containers_and_summaries().await {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(
                        ip = %cred_ip,
                        error = %e,
                        "Failed to get containers from remote Docker host, skipping"
                    );
                    continue;
                }
            };

            if containers.is_empty() {
                tracing::debug!(ip = %cred_ip, "No containers found on remote Docker host");
                continue;
            }

            // For remote Docker, we need subnets for container interface resolution.
            // Use empty subnets — containers on remote hosts typically use bridge networking
            // and their IPs are local to the Docker host, not our network.
            let mut empty_interfaces = Vec::new();
            let containers_interfaces_and_subnets =
                docker_runner.get_container_interfaces(&containers, &[], &mut empty_interfaces);

            match docker_runner
                .scan_and_process_containers(
                    cancel.clone(),
                    containers,
                    &containers_interfaces_and_subnets,
                )
                .await
            {
                Ok(container_data) => {
                    tracing::info!(
                        ip = %cred_ip,
                        discovered = container_data.len(),
                        "Remote Docker scan complete"
                    );
                    remote_count += 1;
                }
                Err(e) => {
                    tracing::error!(
                        ip = %cred_ip,
                        error = %e,
                        "Remote Docker container scanning failed"
                    );
                }
            }
        }

        if remote_count > 0 {
            tracing::info!(
                remote_hosts_scanned = remote_count,
                "Remote Docker scanning complete"
            );
        }

        Ok(())
    }
}

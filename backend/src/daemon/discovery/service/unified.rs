use crate::daemon::discovery::integration::{
    IntegrationContext, IntegrationRegistry, ProbeContext, execute_with_progress_reporting,
};
use crate::daemon::discovery::service::base::DiscoveryRunner;
use crate::daemon::discovery::service::ops::DiscoveryOps;
use crate::daemon::utils::base::{DaemonUtils, merge_host_and_docker_subnets};
use crate::server::bindings::r#impl::base::Binding;
use crate::server::credentials::r#impl::mapping::{
    CredentialMapping, CredentialQueryPayload, CredentialQueryPayloadDiscriminants,
    ResolvedCredential, SnmpCredentialMapping, SnmpQueryCredential,
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
    pub scan_local_docker_socket: bool,
    pub host_naming_fallback: HostNamingFallback,
    pub scan_settings: ScanSettings,
    pub credential_mappings: Vec<CredentialMapping<CredentialQueryPayload>>,
}

// PhaseAllocations removed — replaced by generic integration dispatch.
// Phase 1 (0-5%): Self-report + localhost integrations.
// Phase 2 (5-100%): Network scan with per-host integration probe + execute.

impl DiscoveryRunner<UnifiedDiscovery> {
    pub fn discovery_type(&self) -> DiscoveryType {
        DiscoveryType::Unified {
            host_id: self.domain.host_id,
            subnet_ids: self.domain.subnet_ids.clone(),
            scan_local_docker_socket: self.domain.scan_local_docker_socket,
            host_naming_fallback: self.domain.host_naming_fallback,
            scan_settings: self.domain.scan_settings.clone(),
        }
    }

    pub async fn discover(
        &self,
        request: DaemonDiscoveryRequest,
        cancel: CancellationToken,
    ) -> Result<(), Error> {
        let is_first_run = !self.as_ref().config_store.has_self_reported().await;
        let gateway_ips = self
            .as_ref()
            .utils
            .get_own_routing_table_gateway_ips()
            .await?;
        let ops = DiscoveryOps::new(self.as_ref(), self.discovery_type());

        tracing::info!(
            is_first_run,
            credential_mappings = self.domain.credential_mappings.len(),
            "Unified discovery: self_report=0-5%, network=5-100%",
        );

        // Create subnets before session init (like other runners)
        let created_subnets = match self.discover_create_subnets(&ops, &cancel).await {
            Ok(subnets) => subnets,
            Err(e) => {
                let daemon_id = self.as_ref().config_store.get_id().await?;
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
        let (docker_proxy, docker_proxy_ssl_info, _ssl_temp_handles, _, _) =
            self.resolve_docker_proxy().await.unwrap_or_else(|e| {
                tracing::debug!(error = %e, "Failed to resolve Docker proxy for subnet discovery");
                (Ok(None), Ok(None), Vec::new(), None, None)
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

    // should_run_docker_phase and count_localhost_docker_proxies removed —
    // replaced by generic localhost integration dispatch in run_localhost_integrations().

    /// Extract SNMP credentials from credential_mappings into SnmpCredentialMapping.
    /// Resolves FilePath fields to Value so downstream code doesn't need file I/O.
    /// Caches resolution results per credential to avoid duplicate error logging.
    // TODO: Remove when SNMP scanning is fully extracted into SnmpIntegration.execute()
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
        Option<u16>, // proxy port (None for socket / AppConfig fallback)
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
                let has_ssl = docker_cred.ssl_cert.is_some()
                    && docker_cred.ssl_key.is_some()
                    && docker_cred.ssl_chain.is_some();
                let partial_ssl = !has_ssl
                    && (docker_cred.ssl_cert.is_some()
                        || docker_cred.ssl_key.is_some()
                        || docker_cred.ssl_chain.is_some());
                if partial_ssl {
                    tracing::warn!(
                        "Partial Docker proxy SSL config: all of ssl_cert, ssl_key, and ssl_chain \
                         must be provided for TLS. Falling back to HTTP."
                    );
                }
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

                return Ok((
                    Ok(Some(proxy_url)),
                    ssl_info,
                    temp_handles,
                    cred_id,
                    Some(docker_cred.port),
                ));
            }
        }

        // Fall back to AppConfig with deprecation warning (no credential_id)
        tracing::debug!("No Docker proxy credential in mappings, falling back to AppConfig");
        let docker_proxy = self.as_ref().config_store.get_docker_proxy().await;
        let docker_proxy_ssl_info = self.as_ref().config_store.get_docker_proxy_ssl_info().await;

        Ok((docker_proxy, docker_proxy_ssl_info, Vec::new(), None, None))
    }

    /// Extract all Docker credentials indexed by target IP.
    /// Returns credentials for all IPs that have DockerProxy mappings (both overrides and defaults).
    fn resolve_docker_credentials(
        &self,
    ) -> std::collections::HashMap<
        IpAddr,
        ResolvedCredential<crate::server::credentials::r#impl::mapping::DockerProxyQueryCredential>,
    > {
        let mut result = std::collections::HashMap::new();

        for mapping in &self.domain.credential_mappings {
            for override_entry in &mapping.ip_overrides {
                if let CredentialQueryPayload::DockerProxy(_) = &override_entry.credential {
                    match override_entry.credential.resolve_file_paths() {
                        Ok(CredentialQueryPayload::DockerProxy(resolved)) => {
                            result.insert(
                                override_entry.ip,
                                ResolvedCredential {
                                    credential: resolved,
                                    credential_id: if override_entry.credential_id != Uuid::nil() {
                                        Some(override_entry.credential_id)
                                    } else {
                                        None
                                    },
                                },
                            );
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
            } else if let Err(e) = self.as_ref().config_store.set_has_self_reported().await {
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
        for mapping in &self.domain.credential_mappings {
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
                    utils: &self.as_ref().utils,
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

                // Build a minimal HostData for the daemon host
                // (self-report already created the host — integrations enrich it)
                let mut host_data = crate::daemon::discovery::service::ops::HostData::new(
                    Host::new(HostBase {
                        name: "".to_string(),
                        hostname: None,
                        tags: Vec::new(),
                        network_id: ops.network_id().await?,
                        description: None,
                        source: EntitySource::Discovery { metadata: vec![] },
                        virtualization: None,
                        hidden: false,
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
                    }),
                    vec![],
                    vec![],
                    vec![],
                    vec![],
                );
                host_data.host.id = self.domain.host_id;

                let ctx = IntegrationContext {
                    ip: override_entry.ip,
                    credential,
                    credential_id,
                    cancel,
                    ops,
                    utils: &self.as_ref().utils,
                    probe_handle: probe_result.handle.as_deref(),
                    matched_services: &[],
                    open_ports: &probe_result.ports,
                    endpoint_responses: &[],
                    host_id: self.domain.host_id,
                    host_naming_fallback: self.domain.host_naming_fallback,
                    created_subnets,
                    accept_invalid_certs: self
                        .as_ref()
                        .config_store
                        .get_accept_invalid_scan_certs()
                        .await
                        .unwrap_or(false),
                    scanning_subnet: None,
                };

                tracing::info!(
                    ip = %override_entry.ip,
                    integration = ?discriminant,
                    "Running localhost integration"
                );

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
                    tracing::error!(
                        ip = %override_entry.ip,
                        error = %e,
                        "Localhost integration execute failed"
                    );
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

        ops.create_host(
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

    // run_docker_phase removed — Docker scanning is now handled by DockerIntegration
    // via the generic integration dispatch in run_localhost_integrations (for localhost)
    // and deep_scan_host (for remote hosts).

    /// Network phase: run ARP + deep scan to discover hosts and services
    async fn run_network_phase(
        &self,
        cancel: &CancellationToken,
    ) -> Result<Vec<(IpAddr, Host, super::network::DiscoveredHostData)>, Error> {
        // Network discovery owns subnet resolution — unified just coordinates
        let snmp_credentials = self.extract_snmp_credential_mapping();
        let docker_credentials = self.resolve_docker_credentials();
        let network_discovery = super::network::NetworkScanDiscovery::new(
            self.domain.subnet_ids.clone(),
            self.domain.host_naming_fallback,
            snmp_credentials,
            self.domain.scan_settings.clone(),
            docker_credentials,
            self.domain.credential_mappings.clone(),
        );

        let ops = super::ops::DiscoveryOps::new(self.as_ref(), self.discovery_type());
        let utils = &self.as_ref().utils;

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
        let scan_type = if self.domain.scan_settings.is_full_scan {
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

        let total_credential_mappings = self.domain.credential_mappings.len();

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

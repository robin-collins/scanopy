//! Shared discovery operations used by both the pipeline and integrations.
//!
//! `DiscoveryOps` provides entity creation, service matching, and progress reporting
//! without requiring `DiscoveryRunner<T>` or its associated traits.

use std::{net::IpAddr, sync::Arc, time::Duration};

use anyhow::{Error, anyhow};
use backon::{ExponentialBuilder, Retryable};
use mac_address::MacAddress;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    daemon::{
        discovery::buffer::EntityBuffer,
        shared::{api_client::DaemonApiClient, config::ConfigStore},
    },
    server::{
        credentials::r#impl::types::CredentialAssignment,
        daemons::r#impl::{api::DiscoveryUpdatePayload, base::DaemonMode},
        discovery::r#impl::types::{DiscoveryType, HostNamingFallback},
        hosts::r#impl::{
            api::{DiscoveryHostRequest, HostResponse},
            base::{Host, HostBase},
            virtualization::HostVirtualization,
        },
        if_entries::r#impl::base::IfEntry,
        interfaces::r#impl::base::Interface,
        ports::r#impl::base::{Port, PortType},
        services::{
            definitions::{ServiceDefinitionRegistry, gateway::Gateway},
            r#impl::{
                base::{
                    DiscoverySessionServiceMatchParams, Service, ServiceMatchBaselineParams,
                    ServiceMatchServiceParams,
                },
                definitions::{ServiceDefinition, ServiceDefinitionExt},
                patterns::MatchConfidence,
                virtualization::{DockerVirtualization, ServiceVirtualization},
            },
        },
        shared::{
            types::api::ApiErrorResponse,
            types::entities::{DiscoveryMetadata, EntitySource},
            types::metadata::HasId,
        },
        subnets::r#impl::base::Subnet,
    },
};

use super::base::DaemonDiscoveryService;

/// Default number of retries for entity creation during discovery.
const ENTITY_CREATION_MAX_RETRIES: usize = 5;

/// Timeout for waiting for server confirmation in ServerPoll mode.
const SERVER_POLL_CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(120);

/// Mutable host state passed to integration execute() methods.
/// Integrations enrich the host via builder methods.
pub struct HostData {
    pub host: Host,
    pub services: Vec<Service>,
    pub ports: Vec<Port>,
    pub interfaces: Vec<Interface>,
    pub if_entries: Vec<IfEntry>,
}

impl HostData {
    pub fn new(
        host: Host,
        services: Vec<Service>,
        ports: Vec<Port>,
        interfaces: Vec<Interface>,
        if_entries: Vec<IfEntry>,
    ) -> Self {
        Self {
            host,
            services,
            ports,
            interfaces,
            if_entries,
        }
    }

    // --- Field builders: first-write-wins (only set if currently None) ---

    pub fn with_sys_descr(&mut self, v: String) -> &mut Self {
        if self.host.base.sys_descr.is_none() {
            self.host.base.sys_descr = Some(v);
        }
        self
    }

    pub fn with_sys_name(&mut self, v: String) -> &mut Self {
        if self.host.base.sys_name.is_none() {
            self.host.base.sys_name = Some(v);
        }
        self
    }

    pub fn with_sys_object_id(&mut self, v: String) -> &mut Self {
        if self.host.base.sys_object_id.is_none() {
            self.host.base.sys_object_id = Some(v);
        }
        self
    }

    pub fn with_sys_location(&mut self, v: String) -> &mut Self {
        if self.host.base.sys_location.is_none() {
            self.host.base.sys_location = Some(v);
        }
        self
    }

    pub fn with_sys_contact(&mut self, v: String) -> &mut Self {
        if self.host.base.sys_contact.is_none() {
            self.host.base.sys_contact = Some(v);
        }
        self
    }

    pub fn with_chassis_id(&mut self, v: String) -> &mut Self {
        if self.host.base.chassis_id.is_none() {
            self.host.base.chassis_id = Some(v);
        }
        self
    }

    pub fn with_manufacturer(&mut self, v: String) -> &mut Self {
        if self.host.base.manufacturer.is_none() {
            self.host.base.manufacturer = Some(v);
        }
        self
    }

    pub fn with_model(&mut self, v: String) -> &mut Self {
        if self.host.base.model.is_none() {
            self.host.base.model = Some(v);
        }
        self
    }

    pub fn with_serial_number(&mut self, v: String) -> &mut Self {
        if self.host.base.serial_number.is_none() {
            self.host.base.serial_number = Some(v);
        }
        self
    }

    pub fn with_management_url(&mut self, v: String) -> &mut Self {
        if self.host.base.management_url.is_none() {
            self.host.base.management_url = Some(v);
        }
        self
    }

    pub fn with_virtualization(&mut self, v: HostVirtualization) -> &mut Self {
        if self.host.base.virtualization.is_none() {
            self.host.base.virtualization = Some(v);
        }
        self
    }

    /// Set MAC on the interface matching the given IP address.
    /// Used by SNMP to enrich MAC from ipAddrTable when ARP didn't provide one.
    pub fn with_mac_for_ip(&mut self, ip: IpAddr, mac: MacAddress) -> &mut Self {
        if let Some(interface) = self.interfaces.iter_mut().find(|i| i.base.ip_address == ip)
            && interface.base.mac_address.is_none()
        {
            interface.base.mac_address = Some(mac);
        }
        self
    }

    // --- Append methods: multiple integrations can contribute ---

    pub fn add_service(&mut self, s: Service) -> &mut Self {
        self.services.push(s);
        self
    }

    pub fn add_port(&mut self, p: Port) -> &mut Self {
        self.ports.push(p);
        self
    }

    pub fn add_interface(&mut self, i: Interface) -> &mut Self {
        self.interfaces.push(i);
        self
    }

    pub fn add_if_entry(&mut self, ie: IfEntry) -> &mut Self {
        self.if_entries.push(ie);
        self
    }

    pub fn add_credential_assignment(&mut self, ca: CredentialAssignment) -> &mut Self {
        self.host.base.credential_assignments.push(ca);
        self
    }

    /// Set hostname from SNMP sysName as a fallback if DNS didn't provide one.
    /// Also updates `host.base.name` when it was set to the IP address as a fallback.
    pub fn with_hostname_fallback(&mut self, hostname: String) -> &mut Self {
        if self.host.base.hostname.is_none() {
            // Check if host.name was set to IP as fallback — if so, override with hostname
            let ip_str = self
                .interfaces
                .first()
                .map(|i| i.base.ip_address.to_string());
            if ip_str.as_deref() == Some(&self.host.base.name) {
                self.host.base.name = hostname.clone();
            }
            self.host.base.hostname = Some(hostname);
        }
        self
    }
}

/// Shared discovery operations for both the pipeline and integrations.
///
/// Provides entity creation, service matching, and progress reporting
/// without requiring `DiscoveryRunner<T>`.
pub struct DiscoveryOps {
    pub config_store: Arc<ConfigStore>,
    pub api_client: Arc<DaemonApiClient>,
    pub entity_buffer: Arc<EntityBuffer>,
    pub discovery_type: DiscoveryType,
    session: Arc<tokio::sync::RwLock<Option<super::base::DiscoverySession>>>,
}

impl DiscoveryOps {
    pub fn new(service: &DaemonDiscoveryService, discovery_type: DiscoveryType) -> Self {
        Self {
            config_store: service.config_store.clone(),
            api_client: service.api_client.clone(),
            entity_buffer: service.entity_buffer.clone(),
            discovery_type,
            session: service.current_session.clone(),
        }
    }

    pub async fn daemon_id(&self) -> Result<Uuid, Error> {
        self.config_store.get_id().await
    }

    pub async fn network_id(&self) -> Result<Uuid, Error> {
        self.config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow!("Network ID not set"))
    }

    async fn get_session(&self) -> Result<super::base::DiscoverySession, Error> {
        self.session
            .read()
            .await
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow!("No active discovery session"))
    }

    /// Report scanning progress. Mode-aware: DaemonPoll POSTs to server,
    /// ServerPoll updates session atomics only.
    pub async fn report_progress(&self, percent: u8) -> Result<(), Error> {
        use crate::daemon::discovery::types::base::DiscoverySessionUpdate;
        use std::sync::atomic::Ordering;

        let session = self.get_session().await?;
        let start = session.progress_range_start.load(Ordering::Relaxed);
        let end = session.progress_range_end.load(Ordering::Relaxed);
        let percent = map_progress(percent, start, end);

        let last_report_time = &session.last_progress_report_time;
        let last_progress = &session.last_progress;

        let prev_percent = last_progress.load(Ordering::Relaxed);
        let progress_changed = percent > prev_percent || percent == 100;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let last_time = last_report_time.load(Ordering::Relaxed);

        let heartbeat_interval_secs = 30;
        let heartbeat_due = now >= last_time + heartbeat_interval_secs;

        if !progress_changed && !heartbeat_due && percent < 100 {
            return Ok(());
        }

        if percent < 100 && !heartbeat_due && now < last_time + 10 {
            return Ok(());
        }

        if last_report_time
            .compare_exchange(last_time, now, Ordering::SeqCst, Ordering::Relaxed)
            .is_err()
        {
            return Ok(());
        }

        last_progress.store(percent, Ordering::Relaxed);

        // Mode-aware: only POST in DaemonPoll mode
        let mode = self.config_store.get_mode().await?;
        if mode == DaemonMode::DaemonPoll {
            let update = DiscoverySessionUpdate::scanning(percent);
            let mut payload = DiscoveryUpdatePayload::from_state_and_update(
                self.discovery_type.clone(),
                session.info.clone(),
                update,
            );

            let hosts = session.hosts_discovered.load(Ordering::Relaxed);
            if hosts > 0 {
                payload.hosts_discovered = Some(hosts);
            }
            let estimate = session.estimated_remaining_secs.load(Ordering::Relaxed);
            if estimate != u32::MAX {
                payload.estimated_remaining_secs = Some(estimate);
            }

            let path = format!("/api/v1/discovery/{}/update", session.info.session_id);
            if let Err(e) = self
                .api_client
                .post_no_data(&path, &payload, "Failed to report discovery update")
                .await
            {
                tracing::warn!(
                    session_id = %session.info.session_id,
                    error = %e,
                    "Failed to report discovery update"
                );
            }
        }

        Ok(())
    }

    /// Create a host with its children.
    /// DaemonPoll: POSTs to server. ServerPoll: buffers for server to poll.
    pub async fn create_host(
        &self,
        host: Host,
        interfaces: Vec<Interface>,
        ports: Vec<Port>,
        services: Vec<Service>,
        if_entries: Vec<IfEntry>,
        cancel: &CancellationToken,
    ) -> Result<HostResponse, Error> {
        let mode = self.config_store.get_mode().await?;
        let pending_id = host.id;

        let request = DiscoveryHostRequest {
            host,
            interfaces,
            ports,
            services,
            if_entries,
        };

        self.entity_buffer.push_host(request.clone()).await;

        match mode {
            DaemonMode::DaemonPoll => {
                let api_client = &self.api_client;
                let response: HostResponse = (|| async {
                    api_client
                        .post("/api/v1/hosts/discovery", &request, "Failed to create host")
                        .await
                })
                .retry(
                    ExponentialBuilder::default()
                        .with_min_delay(Duration::from_millis(500))
                        .with_max_delay(Duration::from_secs(30))
                        .with_max_times(ENTITY_CREATION_MAX_RETRIES),
                )
                .when(|e| e.downcast_ref::<ApiErrorResponse>().is_none())
                .notify(|e, dur| tracing::warn!("Retrying host creation after {:?}: {}", dur, e))
                .await?;

                self.entity_buffer
                    .mark_host_created(pending_id, response.clone())
                    .await;

                Ok(response)
            }
            DaemonMode::ServerPoll => {
                let actual_host = self
                    .entity_buffer
                    .await_host(&pending_id, SERVER_POLL_CONFIRMATION_TIMEOUT, cancel)
                    .await
                    .ok_or_else(|| {
                        if cancel.is_cancelled() {
                            anyhow!("Discovery cancelled while waiting for host creation")
                        } else {
                            anyhow!("Timeout waiting for host creation confirmation from server")
                        }
                    })?;

                Ok(HostResponse::from_host_with_children(
                    actual_host,
                    request.interfaces,
                    request.ports,
                    request.services,
                    request.if_entries,
                ))
            }
        }
    }

    /// Create a subnet.
    /// DaemonPoll: POSTs to server. ServerPoll: buffers for server to poll.
    pub async fn create_subnet(
        &self,
        subnet: &Subnet,
        cancel: &CancellationToken,
    ) -> Result<Subnet, Error> {
        let mode = self.config_store.get_mode().await?;
        let pending_id = subnet.id;

        self.entity_buffer.push_subnet(subnet.clone()).await;

        match mode {
            DaemonMode::DaemonPoll => {
                let api_client = &self.api_client;
                let actual: Subnet = (|| async {
                    api_client
                        .post("/api/v1/subnets", subnet, "Failed to create subnet")
                        .await
                })
                .retry(
                    ExponentialBuilder::default()
                        .with_min_delay(Duration::from_millis(500))
                        .with_max_delay(Duration::from_secs(30))
                        .with_max_times(ENTITY_CREATION_MAX_RETRIES),
                )
                .notify(|e, dur| tracing::warn!("Retrying subnet creation after {:?}: {}", dur, e))
                .await?;

                self.entity_buffer
                    .mark_subnet_created(pending_id, actual.clone())
                    .await;

                Ok(actual)
            }
            DaemonMode::ServerPoll => self
                .entity_buffer
                .await_subnet(&pending_id, SERVER_POLL_CONFIRMATION_TIMEOUT, cancel)
                .await
                .ok_or_else(|| {
                    if cancel.is_cancelled() {
                        anyhow!("Discovery cancelled while waiting for subnet creation")
                    } else {
                        anyhow!("Timeout waiting for subnet creation confirmation from server")
                    }
                }),
        }
    }

    /// Run service matching against all registered service definitions.
    /// Returns matched services and ports. Pure logic — no side effects.
    pub fn match_services(
        &self,
        host: &Host,
        baseline_params: &ServiceMatchBaselineParams,
        gateway_ips: &[IpAddr],
        daemon_id: &Uuid,
        network_id: &Uuid,
    ) -> Result<(Vec<Service>, Vec<Port>), Error> {
        use crate::server::services::definitions::{
            docker_container::DockerContainer, open_ports::OpenPorts,
        };

        let ServiceMatchBaselineParams { all_ports, .. } = baseline_params;

        let mut services = Vec::new();
        let mut host_ports = Vec::new();
        let mut unbound_ports = all_ports.to_vec();
        let mut container_matched = false;

        let mut sorted_service_definitions: Vec<Box<dyn ServiceDefinition>> =
            ServiceDefinitionRegistry::all_service_definitions()
                .into_iter()
                .collect();

        sorted_service_definitions.sort_by_key(|s| {
            if !ServiceDefinitionExt::is_generic(s) {
                0
            } else if s.id() == OpenPorts.id() {
                3
            } else if s.id() == DockerContainer.id() || s.id() == Gateway.id() {
                2
            } else {
                1
            }
        });

        for service_definition in sorted_service_definitions {
            let service_params = ServiceMatchServiceParams {
                service_definition,
                matched_services: &services,
                unbound_ports: &unbound_ports,
            };

            let params: DiscoverySessionServiceMatchParams<'_> =
                DiscoverySessionServiceMatchParams {
                    service_params,
                    baseline_params,
                    daemon_id,
                    discovery_type: &self.discovery_type,
                    network_id,
                    gateway_ips,
                    host_id: &host.id,
                };

            if let Some((service, mut ports, _endpoint)) = Service::from_discovery(params)
                && !container_matched
            {
                if let Some(ServiceVirtualization::Docker(DockerVirtualization {
                    container_id: Some(_),
                    ..
                })) = &service.base.virtualization
                {
                    container_matched = true
                }

                let bound_port_types: Vec<PortType> =
                    ports.iter().map(|p| p.base.port_type).collect();

                host_ports.append(&mut ports);
                unbound_ports.retain(|p| !bound_port_types.contains(p));
                services.push(service);
            }
        }

        services.sort_by_key(|a| {
            -(match &a.base.source {
                EntitySource::DiscoveryWithMatch { details, .. } => {
                    (details.confidence as i32)
                        + if a.base.service_definition.has_logo() {
                            1
                        } else {
                            0
                        }
                }
                _ => MatchConfidence::NotApplicable as i32,
            })
        });

        host_ports.extend(unbound_ports.into_iter().map(Port::new_hostless));

        Ok((services, host_ports))
    }

    /// Build a HostData from scan results: creates host entity, runs service matching, names host.
    pub async fn build_host_from_scan(
        &self,
        params: ServiceMatchBaselineParams<'_>,
        hostname: Option<String>,
        host_naming_fallback: HostNamingFallback,
    ) -> Result<Option<HostData>, Error> {
        let ServiceMatchBaselineParams { interface, .. } = params;

        let daemon_id = self.daemon_id().await?;
        let network_id = self.network_id().await?;
        let session = self.get_session().await?;
        let gateway_ips = session.gateway_ips.clone();

        let mut host = Host::new(HostBase {
            name: "Unknown Device".to_string(),
            hostname: hostname.clone(),
            tags: Vec::new(),
            network_id,
            description: None,
            source: EntitySource::Discovery {
                metadata: vec![DiscoveryMetadata::new(
                    self.discovery_type.clone(),
                    daemon_id,
                )],
            },
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
        });

        let interfaces = vec![interface.clone()];

        let (services, ports) =
            self.match_services(&host, &params, &gateway_ips, &daemon_id, &network_id)?;

        // Determine host name
        let best_service_name = services
            .iter()
            .find(|s| !ServiceDefinitionExt::is_generic(&s.base.service_definition))
            .map(|s| s.base.service_definition.name().to_string());

        if let Some(hostname) = hostname {
            host.base.name = hostname;
        } else if host_naming_fallback == HostNamingFallback::BestService
            && let Some(best_service_name) = best_service_name
        {
            host.base.name = best_service_name
        } else if host_naming_fallback == HostNamingFallback::Ip {
            host.base.name = interface.base.ip_address.to_string()
        } else if let Some(best_service_name) = best_service_name {
            host.base.name = best_service_name
        } else {
            host.base.name = interface.base.ip_address.to_string()
        }

        tracing::info!(
            ip = %interface.base.ip_address,
            host_name = %host.base.name,
            service_count = %services.len(),
            port_count = %ports.len(),
            "Processed host",
        );

        Ok(Some(HostData::new(
            host,
            services,
            ports,
            interfaces,
            vec![],
        )))
    }
}

fn map_progress(raw: u8, start: u8, end: u8) -> u8 {
    if start == 0 && end == 100 {
        return raw;
    }
    start + (raw as f64 * (end - start) as f64 / 100.0) as u8
}

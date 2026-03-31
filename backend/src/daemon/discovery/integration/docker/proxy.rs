//! Docker proxy transport — connects to remote Docker daemon via HTTP(S) proxy.
//!
//! Contains probe/execute logic, proxy URL construction, SSL resolution,
//! and credential resolution helpers used by the unified discovery orchestrator.

use std::net::IpAddr;
use std::time::Duration;

use anyhow::{Error, Result};
use bollard::Docker;

use crate::daemon::utils::base::DaemonUtils;
use crate::server::credentials::r#impl::mapping::{
    CredentialMapping, CredentialQueryPayload, DockerProxyQueryCredential, ResolvedCredential,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

use super::{DockerProbeHandle, DOCKER_PROBE_MAX_ATTEMPTS};
use crate::daemon::discovery::integration::{ProbeContext, ProbeFailure, ProbeSuccess};

use crate::daemon::shared::config::ConfigStore;

use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Build a proxy URL from Docker credential and target IP.
pub fn build_docker_proxy_url(ip: IpAddr, cred: &DockerProxyQueryCredential) -> String {
    let proxy_path = cred.path.as_deref().unwrap_or("").trim_start_matches('/');
    let has_ssl = cred.ssl_cert.is_some();
    let scheme = if has_ssl { "https" } else { "http" };
    let host_str = match ip {
        IpAddr::V6(v6) => format!("[{}]", v6),
        _ => ip.to_string(),
    };
    if proxy_path.is_empty() {
        format!("{}://{}:{}", scheme, host_str, cred.port)
    } else {
        format!("{}://{}:{}/{}", scheme, host_str, cred.port, proxy_path)
    }
}

pub type SslPaths = Option<(String, String, String)>;

/// Resolve SSL paths from credential, returning (cert_path, key_path, chain_path)
/// and temp file handles that must be kept alive until the Docker client is dropped.
pub fn resolve_docker_ssl(
    cred: &DockerProxyQueryCredential,
) -> Result<(SslPaths, Vec<tempfile::NamedTempFile>), Error> {
    let label = "Docker proxy connection";
    let mut temp_handles = Vec::new();

    let ssl_info = if let (Some(cert_rv), Some(key_rv), Some(chain_rv)) =
        (&cred.ssl_cert, &cred.ssl_key, &cred.ssl_chain)
    {
        let (cert_path, cert_handle) = cert_rv.resolve_to_path("ssl_cert", label)?;
        let (key_path, key_handle) = key_rv.resolve_to_path("ssl_key", label)?;
        let (chain_path, chain_handle) = chain_rv.resolve_to_path("ssl_chain", label)?;
        temp_handles.extend(cert_handle);
        temp_handles.extend(key_handle);
        temp_handles.extend(chain_handle);
        Some((
            cert_path.to_string_lossy().into_owned(),
            key_path.to_string_lossy().into_owned(),
            chain_path.to_string_lossy().into_owned(),
        ))
    } else {
        None
    };

    Ok((ssl_info, temp_handles))
}

/// Probe a Docker proxy endpoint.
pub async fn probe(ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure> {
    let docker_cred = match ctx.credential {
        CredentialQueryPayload::DockerProxy(cred) => cred,
        _ => {
            return Err(ProbeFailure {
                message: "Expected DockerProxy credential".to_string(),
            });
        }
    };

    let proxy_url = build_docker_proxy_url(ctx.ip, docker_cred);
    let (ssl_paths, ssl_temp_handles) =
        resolve_docker_ssl(docker_cred).map_err(|e| ProbeFailure {
            message: format!("Failed to resolve Docker SSL: {}", e),
        })?;

    tracing::info!(ip = %ctx.ip, proxy_url = %proxy_url, "Attempting Docker proxy probe");

    for attempt in 1..=DOCKER_PROBE_MAX_ATTEMPTS {
        if ctx.cancel.is_cancelled() {
            return Err(ProbeFailure {
                message: "Cancelled".to_string(),
            });
        }

        match ctx
            .utils
            .new_docker_client(Ok(Some(proxy_url.clone())), Ok(ssl_paths.clone()))
            .await
        {
            Ok(client) => {
                tracing::info!(
                    ip = %ctx.ip,
                    proxy_url = %proxy_url,
                    "Docker client probe succeeded"
                );
                return Ok(ProbeSuccess {
                    client_probe: ClientProbe::Docker,
                    ports: vec![PortType::new_tcp(docker_cred.port)],
                    handle: Some(Box::new(DockerProbeHandle {
                        client,
                        port: docker_cred.port,
                        _ssl_temp_handles: ssl_temp_handles,
                    })),
                });
            }
            Err(e) => {
                if attempt < DOCKER_PROBE_MAX_ATTEMPTS {
                    tracing::debug!(
                        ip = %ctx.ip,
                        attempt = attempt,
                        error = %e,
                        "Docker client probe failed, retrying"
                    );
                    tokio::time::sleep(Duration::from_secs(2)).await;
                } else {
                    return Err(ProbeFailure {
                        message: format!(
                            "Docker probe failed after {} attempts: {}",
                            DOCKER_PROBE_MAX_ATTEMPTS, e
                        ),
                    });
                }
            }
        }
    }

    Err(ProbeFailure {
        message: "Docker probe exhausted all attempts".to_string(),
    })
}

/// Execute Docker container scanning after a successful probe.
pub async fn execute(
    ctx: &crate::daemon::discovery::integration::IntegrationContext<'_>,
    host_data: &mut crate::daemon::discovery::service::ops::HostData,
) -> Result<(), Error> {
    use std::sync::Arc;
    use std::sync::atomic::AtomicU8;

    use super::scanner::DockerScanner;

    // Downcast probe handle to DockerProbeHandle
    let handle = ctx
        .probe_handle
        .and_then(|h| h.downcast_ref::<DockerProbeHandle>())
        .ok_or_else(|| anyhow::anyhow!("Missing DockerProbeHandle"))?;

    // Find the Docker daemon service from host_data
    let docker_service_id = host_data
        .services
        .iter()
        .find(|s| {
            s.base
                .service_definition
                .name()
                .eq_ignore_ascii_case("docker")
        })
        .map(|s| s.id)
        .ok_or_else(|| anyhow::anyhow!("Docker daemon service not found in host_data"))?;

    let scanner = DockerScanner {
        docker_client: &handle.client,
        docker_service_id,
        host_ip: ctx.ip,
        host_naming_fallback: ctx.host_naming_fallback,
        ops: ctx.ops,
        cancel: ctx.cancel,
        accept_invalid_certs: ctx.accept_invalid_certs,
        utils: ctx.utils,
    };

    // Create Docker bridge subnets locally (sent to server via create_host after service dedup)
    let bridge_subnets = scanner.create_docker_bridge_subnets().await?;
    for subnet in &bridge_subnets {
        host_data.add_subnet(subnet.clone());
    }
    ctx.ops.report_progress(10).await.ok();

    // Combine created_subnets (from pipeline) with bridge subnets for interface resolution
    let all_subnets: Vec<_> = ctx
        .created_subnets
        .iter()
        .cloned()
        .chain(bridge_subnets)
        .collect();

    // Get containers
    let containers = scanner.get_containers_and_summaries().await?;
    let container_count = containers.len();
    ctx.ops.report_progress(20).await.ok();

    // Build interface map from containers + subnets
    let mut host_interfaces = host_data.interfaces.clone();
    let containers_interfaces_and_subnets =
        scanner.get_container_interfaces(&containers, &all_subnets, &mut host_interfaces);

    // Scan and process all containers
    let container_results = scanner
        .scan_and_process_containers(
            containers,
            &containers_interfaces_and_subnets,
            Arc::new(AtomicU8::new(0)),
        )
        .await?;
    ctx.ops.report_progress(90).await.ok();

    tracing::info!(
        discovered = %container_results.len(),
        total_containers = container_count,
        "Docker container scanning complete"
    );

    // Add container services/ports/interfaces to host_data
    for result in container_results {
        for service in result.services {
            host_data.add_service(service);
        }
        for port in result.ports {
            host_data.add_port(port);
        }
        for interface in result.interfaces {
            host_data.add_interface(interface);
        }
    }

    Ok(())
}

// ============================================================================
// Credential resolution helpers (moved from unified.rs)
// ============================================================================

/// Resolve the Docker proxy configuration for localhost from credential mappings.
///
/// Checks credential_mappings for DockerProxy targeting localhost only.
/// Remote Docker credentials are handled in deep_scan_host() during network scanning.
/// Falls back to AppConfig if no credential found.
pub async fn resolve_docker_proxy(
    credential_mappings: &[CredentialMapping<CredentialQueryPayload>],
    config_store: &ConfigStore,
) -> Result<(
    Result<Option<String>, Error>,
    Result<Option<(String, String, String)>, Error>,
    Vec<tempfile::NamedTempFile>,
    Option<Uuid>,
    Option<u16>,
)> {
    for mapping in credential_mappings {
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
            (Some(d), None, None)
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
                let (chain_path, chain_handle) = chain_rv.resolve_to_path("ssl_chain", label)?;
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
    let docker_proxy = config_store.get_docker_proxy().await;
    let docker_proxy_ssl_info = config_store.get_docker_proxy_ssl_info().await;

    Ok((docker_proxy, docker_proxy_ssl_info, Vec::new(), None, None))
}

/// Extract all Docker credentials indexed by target IP.
/// Returns credentials for all IPs that have DockerProxy mappings.
pub fn resolve_docker_credentials(
    credential_mappings: &[CredentialMapping<CredentialQueryPayload>],
) -> HashMap<IpAddr, ResolvedCredential<DockerProxyQueryCredential>> {
    let mut result = HashMap::new();

    for mapping in credential_mappings {
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

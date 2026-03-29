//! Docker discovery integration.
//!
//! Probe: connects Docker client via proxy URL from credential.
//! Execute: scans containers, adds container services/interfaces/ports to HostData.

use std::time::Duration;

use anyhow::Error;
use async_trait::async_trait;
use bollard::Docker;

use crate::daemon::utils::base::DaemonUtils;
use crate::server::{
    credentials::r#impl::mapping::{
        CredentialQueryPayload, CredentialQueryPayloadDiscriminants, DockerProxyQueryCredential,
    },
    ports::r#impl::base::PortType,
    services::r#impl::patterns::ClientProbe,
};

use super::{DiscoveryIntegration, IntegrationContext, ProbeContext, ProbeFailure, ProbeSuccess};
use crate::daemon::discovery::service::ops::HostData;

const DOCKER_PROBE_MAX_ATTEMPTS: u32 = 3;

pub struct DockerIntegration;

/// Build a proxy URL from Docker credential and target IP.
fn build_docker_proxy_url(ip: std::net::IpAddr, cred: &DockerProxyQueryCredential) -> String {
    let proxy_path = cred.path.as_deref().unwrap_or("").trim_start_matches('/');
    let has_ssl = cred.ssl_cert.is_some();
    let scheme = if has_ssl { "https" } else { "http" };
    let host_str = match ip {
        std::net::IpAddr::V6(v6) => format!("[{}]", v6),
        _ => ip.to_string(),
    };
    if proxy_path.is_empty() {
        format!("{}://{}:{}", scheme, host_str, cred.port)
    } else {
        format!("{}://{}:{}/{}", scheme, host_str, cred.port, proxy_path)
    }
}

type SslPaths = Option<(String, String, String)>;

/// Resolve SSL paths from credential, returning (cert_path, key_path, chain_path)
/// and temp file handles that must be kept alive until the Docker client is dropped.
fn resolve_docker_ssl(
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

/// Handle returned by a successful Docker probe.
pub struct DockerProbeHandle {
    pub client: Docker,
    pub port: u16,
    /// Must stay alive until client is dropped — bollard reads certs lazily.
    pub _ssl_temp_handles: Vec<tempfile::NamedTempFile>,
}

#[async_trait]
impl DiscoveryIntegration for DockerIntegration {
    fn credential_type(&self) -> CredentialQueryPayloadDiscriminants {
        CredentialQueryPayloadDiscriminants::DockerProxy
    }

    fn estimated_seconds(&self) -> u32 {
        5
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(300)
    }

    fn probe_gate_ports(&self, credential: &CredentialQueryPayload) -> Vec<PortType> {
        match credential {
            CredentialQueryPayload::DockerProxy(docker) => {
                vec![PortType::new_tcp(docker.port)]
            }
            _ => vec![],
        }
    }

    async fn probe(&self, ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure> {
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

    async fn execute(
        &self,
        _ctx: &IntegrationContext<'_>,
        _host_data: &mut HostData,
    ) -> Result<(), Error> {
        // TODO(Step 6): Extract Docker container scanning
        // - Downcast ctx.probe_handle to DockerProbeHandle
        // - Find docker_service_id from host_data.services
        // - Create bridge subnets
        // - For each container: scan, match services, add to host_data
        Ok(())
    }
}

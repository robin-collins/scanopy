//! Docker local socket discovery integration.
//!
//! Connects to the Docker daemon via the local Unix socket (/var/run/docker.sock)
//! without a proxy. Auto-injected by the daemon when socket access is available.
//! Shares `DockerScanner` with `DockerIntegration` for container scanning.

use std::time::Duration;

use anyhow::Error;
use async_trait::async_trait;

use crate::{
    daemon::utils::base::DaemonUtils,
    server::{
        credentials::r#impl::mapping::CredentialQueryPayloadDiscriminants,
        services::r#impl::patterns::ClientProbe,
    },
};

use super::docker::DockerProbeHandle;
use super::{DiscoveryIntegration, IntegrationContext, ProbeContext, ProbeFailure, ProbeSuccess};
use crate::daemon::discovery::service::ops::HostData;

pub struct DockerSocketIntegration;

#[async_trait]
impl DiscoveryIntegration for DockerSocketIntegration {
    fn credential_type(&self) -> CredentialQueryPayloadDiscriminants {
        CredentialQueryPayloadDiscriminants::DockerSocket
    }

    fn estimated_seconds(&self) -> u32 {
        5
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(300)
    }

    // No probe_gate_ports — Unix socket, no TCP port needed.

    async fn probe(&self, ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure> {
        // Connect to local Docker socket (no proxy URL, no SSL)
        match ctx.utils.new_docker_client(Ok(None), Ok(None)).await {
            Ok(client) => {
                tracing::info!("Docker socket probe succeeded (local socket)");
                Ok(ProbeSuccess {
                    client_probe: ClientProbe::Docker,
                    ports: vec![],
                    handle: Some(Box::new(DockerProbeHandle {
                        client,
                        port: 0, // No port for Unix socket
                        _ssl_temp_handles: vec![],
                    })),
                })
            }
            Err(e) => Err(ProbeFailure {
                message: format!("Docker socket connection failed: {}", e),
            }),
        }
    }

    async fn execute(
        &self,
        ctx: &IntegrationContext<'_>,
        host_data: &mut HostData,
    ) -> Result<(), Error> {
        // Reuse DockerIntegration's execute — same container scanning logic
        super::docker::DockerIntegration
            .execute(ctx, host_data)
            .await
    }
}

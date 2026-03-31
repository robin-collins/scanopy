//! Docker discovery integration.
//!
//! Two transport mechanisms:
//! - `proxy` — connects via HTTP(S) proxy URL (remote or local)
//! - `socket` — connects via local Unix socket (/var/run/docker.sock)
//!
//! Both share the same container scanning logic in `scanner`.

pub mod proxy;
mod scanner;
pub mod socket;

use std::time::Duration;

use anyhow::Error;
use async_trait::async_trait;
use bollard::Docker;

use crate::server::{
    credentials::r#impl::mapping::{CredentialQueryPayload, CredentialQueryPayloadDiscriminants},
    ports::r#impl::base::PortType,
};

use super::{DiscoveryIntegration, IntegrationContext, ProbeContext, ProbeFailure, ProbeSuccess};
use crate::daemon::discovery::service::ops::HostData;

const DOCKER_PROBE_MAX_ATTEMPTS: u32 = 3;

pub struct DockerIntegration;

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
        proxy::probe(ctx).await
    }

    async fn execute(
        &self,
        ctx: &IntegrationContext<'_>,
        host_data: &mut HostData,
    ) -> Result<(), Error> {
        proxy::execute(ctx, host_data).await
    }
}

//! SNMP discovery integration.
//!
//! Probe: credentialed SNMP check on UDP ports 161/1161.
//! Execute: walks ifTable, queries LLDP/CDP/ARP/Entity-MIB/Bridge-FDB,
//!          enriches HostData with system info, interfaces, and if_entries.

use std::time::Duration;

use anyhow::Error;
use async_trait::async_trait;

use crate::{
    daemon::utils::scanner::{try_snmp_with_credential_on_port, try_snmp_with_public_on_port},
    server::{
        credentials::r#impl::mapping::{
            CredentialQueryPayload, CredentialQueryPayloadDiscriminants, SnmpQueryCredential,
        },
        ports::r#impl::base::PortType,
        services::r#impl::patterns::ClientProbe,
    },
};

use super::{DiscoveryIntegration, IntegrationContext, ProbeContext, ProbeFailure, ProbeSuccess};
use crate::daemon::discovery::service::ops::HostData;

/// Handle returned by a successful SNMP probe — carries the working credential and port.
pub struct SnmpProbeHandle {
    pub credential: SnmpQueryCredential,
    pub port: u16,
}

pub struct SnmpIntegration;

#[async_trait]
impl DiscoveryIntegration for SnmpIntegration {
    fn credential_type(&self) -> CredentialQueryPayloadDiscriminants {
        CredentialQueryPayloadDiscriminants::Snmp
    }

    fn estimated_seconds(&self) -> u32 {
        4
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(30)
    }

    // No probe_gate_ports — SNMP does its own UDP port probing.

    async fn probe(&self, ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure> {
        let snmp_cred = match ctx.credential {
            CredentialQueryPayload::Snmp(cred) => cred,
            _ => {
                return Err(ProbeFailure {
                    message: "Expected SNMP credential".to_string(),
                });
            }
        };

        let snmp_ports: &[u16] = &[161, 1161];

        // Try the provided credential on each SNMP port
        for &port in snmp_ports {
            if ctx.cancel.is_cancelled() {
                return Err(ProbeFailure {
                    message: "Cancelled".to_string(),
                });
            }

            match try_snmp_with_credential_on_port(ctx.ip, snmp_cred, port).await {
                Ok(Some(detected_port)) => {
                    return Ok(ProbeSuccess {
                        client_probe: ClientProbe::Snmp,
                        ports: vec![PortType::new_udp(detected_port)],
                        handle: Some(Box::new(SnmpProbeHandle {
                            credential: snmp_cred.clone(),
                            port: detected_port,
                        })),
                    });
                }
                Ok(None) => continue,
                Err(e) => {
                    tracing::debug!(
                        ip = %ctx.ip,
                        port = port,
                        error = %e,
                        "SNMP credential probe failed"
                    );
                }
            }
        }

        // Last resort: try "public" community on each port
        for &port in snmp_ports {
            if ctx.cancel.is_cancelled() {
                return Err(ProbeFailure {
                    message: "Cancelled".to_string(),
                });
            }

            match try_snmp_with_public_on_port(ctx.ip, port).await {
                Ok(Some(detected_port)) => {
                    // Build a "public" credential for the handle
                    let public_cred = SnmpQueryCredential {
                        version: snmp_cred.version,
                        community:
                            crate::server::credentials::r#impl::mapping::ResolvableSecret::Value {
                                value: "public".to_string(),
                            },
                    };
                    return Ok(ProbeSuccess {
                        client_probe: ClientProbe::Snmp,
                        ports: vec![PortType::new_udp(detected_port)],
                        handle: Some(Box::new(SnmpProbeHandle {
                            credential: public_cred,
                            port: detected_port,
                        })),
                    });
                }
                Ok(None) => continue,
                Err(e) => {
                    tracing::debug!(
                        ip = %ctx.ip,
                        port = port,
                        error = %e,
                        "SNMP public community probe failed"
                    );
                }
            }
        }

        Err(ProbeFailure {
            message: format!("SNMP not responding on {} with any credential", ctx.ip),
        })
    }

    async fn execute(
        &self,
        _ctx: &IntegrationContext<'_>,
        _host_data: &mut HostData,
    ) -> Result<(), Error> {
        // TODO(Step 6): Extract SNMP scanning from deep_scan_host
        // - Downcast ctx.probe_handle to SnmpProbeHandle
        // - Walk ifTable, query LLDP/CDP/ARP/Entity-MIB/Bridge-FDB/lldp_local
        // - Enrich host_data via builder methods
        // - Create remote subnets + ARP hosts via ctx.ops
        Ok(())
    }
}

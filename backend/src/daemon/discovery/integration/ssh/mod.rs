//! Read-only SSH discovery integration.
//!
//! Adapted from the CodexNet collector design under AGPL-3.0 with permission from its author.
//! Commands are selected only from a platform-specific static allowlist; credentials and command
//! output are never logged.

use std::{path::PathBuf, sync::Arc, time::Duration};

use anyhow::{Error, anyhow};
use async_trait::async_trait;
use russh::{ChannelMsg, Disconnect, client, keys};
use tokio::sync::Mutex;

use crate::server::{
    credentials::r#impl::mapping::{
        CredentialQueryPayload, CredentialQueryPayloadDiscriminants, ResolvableSecret,
        SshAuthentication, SshHostKeyPolicy, SshPlatform, SshQueryCredential,
    },
    ports::r#impl::base::PortType,
    services::r#impl::patterns::ClientProbe,
};

use super::{DiscoveryIntegration, IntegrationContext, ProbeContext, ProbeFailure, ProbeSuccess};
use crate::daemon::discovery::service::ops::HostData;

const COMMAND_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_COMMAND_OUTPUT: usize = 1024 * 1024;
const MAX_STORED_DESCRIPTION: usize = 64 * 1024;

const LINUX_COMMANDS: &[&str] = &[
    "hostname",
    "uname -a",
    "cat /etc/os-release",
    "ip -brief address",
    "ip route show",
    "systemd-detect-virt",
];

const CISCO_IOS_COMMANDS: &[&str] = &[
    "show version",
    "show inventory",
    "show interfaces status",
    "show interfaces",
    "show vlan brief",
    "show mac address-table",
    "show ip arp",
    "show lldp neighbors detail",
    "show cdp neighbors detail",
    "show power inline",
    "show environment all",
];

const HP_COMWARE_COMMANDS: &[&str] = &[
    "display version",
    "display device manuinfo",
    "display interface brief",
    "display interface",
    "display vlan all",
    "display mac-address",
    "display arp",
    "display lldp neighbor-information verbose",
    "display poe device",
    "display environment",
];

const ARUBA_AOS_COMMANDS: &[&str] = &[
    "show system",
    "show version",
    "show modules",
    "show interfaces brief",
    "show interfaces",
    "show vlans",
    "show mac-address",
    "show arp",
    "show lldp info remote-device detail",
    "show power-over-ethernet brief",
    "show system temperature",
];

fn commands(platform: SshPlatform) -> &'static [&'static str] {
    match platform {
        SshPlatform::Linux => LINUX_COMMANDS,
        SshPlatform::CiscoIos => CISCO_IOS_COMMANDS,
        SshPlatform::HpComware => HP_COMWARE_COMMANDS,
        SshPlatform::ArubaAos => ARUBA_AOS_COMMANDS,
    }
}

fn port_type(port: u16) -> PortType {
    if port == PortType::Ssh.number() {
        PortType::Ssh
    } else {
        PortType::new_tcp(port)
    }
}

#[derive(Clone)]
struct SshClientHandler {
    host: String,
    port: u16,
    host_key_policy: SshHostKeyPolicy,
    known_hosts_file: Option<PathBuf>,
}

impl client::Handler for SshClientHandler {
    type Error = anyhow::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        match self.host_key_policy {
            SshHostKeyPolicy::AcceptUnknown => Ok(true),
            SshHostKeyPolicy::Strict => {
                let path = self
                    .known_hosts_file
                    .as_ref()
                    .ok_or_else(|| anyhow!("strict SSH host-key policy has no known_hosts file"))?;
                keys::check_known_hosts_path(&self.host, self.port, server_public_key, path)
                    .map_err(Error::from)
            }
        }
    }
}

struct SshProbeHandle {
    session: Mutex<client::Handle<SshClientHandler>>,
    platform: SshPlatform,
}

pub struct SshIntegration;

#[async_trait]
impl DiscoveryIntegration for SshIntegration {
    fn credential_type(&self) -> CredentialQueryPayloadDiscriminants {
        CredentialQueryPayloadDiscriminants::Ssh
    }

    fn estimated_seconds(&self) -> u32 {
        20
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(180)
    }

    fn probe_gate_ports(&self, credential: &CredentialQueryPayload) -> Vec<PortType> {
        match credential {
            CredentialQueryPayload::Ssh(ssh) => vec![port_type(ssh.port)],
            _ => vec![],
        }
    }

    async fn probe(&self, ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure> {
        let credential = match ctx.credential {
            CredentialQueryPayload::Ssh(credential) => credential,
            _ => return Err(failure("expected SSH credential")),
        };

        if ctx.cancel.is_cancelled() {
            return Err(failure("cancelled"));
        }

        let session = connect(ctx.ip.to_string(), credential)
            .await
            .map_err(|error| {
                tracing::debug!(ip = %ctx.ip, error = %error, "SSH credential probe failed");
                failure("SSH connection, host verification, or authentication failed")
            })?;

        Ok(ProbeSuccess {
            client_probe: ClientProbe::Ssh,
            ports: vec![port_type(credential.port)],
            handle: Some(Box::new(SshProbeHandle {
                session: Mutex::new(session),
                platform: credential.platform,
            })),
        })
    }

    async fn execute(
        &self,
        ctx: &IntegrationContext<'_>,
        host_data: &mut HostData,
    ) -> Result<(), Error> {
        let handle = ctx
            .probe_handle
            .and_then(|value| value.downcast_ref::<SshProbeHandle>())
            .ok_or_else(|| anyhow!("SSH execute called without a probe handle"))?;
        let mut session = handle.session.lock().await;
        let mut successful = Vec::new();

        for command in commands(handle.platform) {
            if ctx.cancel.is_cancelled() {
                return Err(anyhow!("discovery was cancelled"));
            }
            match tokio::time::timeout(COMMAND_TIMEOUT, execute_command(&mut session, command))
                .await
            {
                Ok(Ok(output)) if !output.trim().is_empty() => successful.push((*command, output)),
                Ok(Ok(_)) => {}
                Ok(Err(error)) => {
                    tracing::debug!(ip = %ctx.ip, command, error = %error, "Read-only SSH command failed");
                }
                Err(_) => {
                    tracing::debug!(ip = %ctx.ip, command, "Read-only SSH command timed out");
                }
            }
        }

        if handle.platform == SshPlatform::Linux
            && let Some((_, hostname)) = successful
                .iter()
                .find(|(command, _)| *command == "hostname")
        {
            let hostname = hostname.trim();
            if !hostname.is_empty() {
                host_data.with_hostname_fallback(hostname.to_string());
                host_data.with_sys_name(hostname.to_string());
            }
        }

        let description = successful
            .iter()
            .filter(|(command, _)| {
                matches!(
                    *command,
                    "uname -a"
                        | "cat /etc/os-release"
                        | "show version"
                        | "display version"
                        | "show system"
                )
            })
            .map(|(command, output)| format!("$ {command}\n{}", output.trim()))
            .collect::<Vec<_>>()
            .join("\n\n");
        if !description.is_empty() {
            host_data.with_sys_descr(truncate_utf8(description, MAX_STORED_DESCRIPTION));
        }

        let _ = session
            .disconnect(Disconnect::ByApplication, "", "English")
            .await;
        Ok(())
    }
}

async fn connect(
    host: String,
    credential: &SshQueryCredential,
) -> Result<client::Handle<SshClientHandler>, Error> {
    let config = Arc::new(client::Config {
        inactivity_timeout: Some(Duration::from_secs(15)),
        ..Default::default()
    });
    let handler = SshClientHandler {
        host: host.clone(),
        port: credential.port,
        host_key_policy: credential.host_key_policy,
        known_hosts_file: credential.known_hosts_file.as_ref().map(PathBuf::from),
    };
    let mut session = client::connect(config, (host.as_str(), credential.port), handler).await?;

    let authenticated = match &credential.authentication {
        SshAuthentication::Password { password } => session
            .authenticate_password(&credential.username, resolved_secret(password)?)
            .await?
            .success(),
        SshAuthentication::PrivateKey {
            private_key,
            passphrase,
        } => {
            let key = keys::decode_secret_key(
                resolved_secret(private_key)?,
                passphrase.as_ref().map(resolved_secret).transpose()?,
            )?;
            let hash = session.best_supported_rsa_hash().await?.flatten();
            session
                .authenticate_publickey(
                    &credential.username,
                    keys::PrivateKeyWithHashAlg::new(Arc::new(key), hash),
                )
                .await?
                .success()
        }
    };
    if !authenticated {
        return Err(anyhow!("SSH authentication rejected"));
    }
    Ok(session)
}

fn resolved_secret(secret: &ResolvableSecret) -> Result<&str, Error> {
    match secret {
        ResolvableSecret::Value { value } => Ok(value),
        ResolvableSecret::FilePath { .. } => Err(anyhow!("SSH secret file was not resolved")),
    }
}

async fn execute_command(
    session: &mut client::Handle<SshClientHandler>,
    command: &str,
) -> Result<String, Error> {
    debug_assert!(
        commands(SshPlatform::Linux).contains(&command)
            || commands(SshPlatform::CiscoIos).contains(&command)
            || commands(SshPlatform::HpComware).contains(&command)
            || commands(SshPlatform::ArubaAos).contains(&command)
    );
    let mut channel = session.channel_open_session().await?;
    channel.exec(true, command).await?;
    let mut output = Vec::new();
    while let Some(message) = channel.wait().await {
        match message {
            ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. } => {
                if output.len().saturating_add(data.len()) > MAX_COMMAND_OUTPUT {
                    return Err(anyhow!("SSH command output exceeded the configured limit"));
                }
                output.extend_from_slice(data.as_ref());
            }
            _ => {}
        }
    }
    Ok(String::from_utf8_lossy(&output).into_owned())
}

fn truncate_utf8(mut value: String, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value;
    }
    let mut boundary = max_bytes;
    while !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    value.truncate(boundary);
    value
}

fn failure(message: impl Into<String>) -> ProbeFailure {
    ProbeFailure {
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_platform_has_a_non_empty_fixed_allowlist() {
        for platform in [
            SshPlatform::Linux,
            SshPlatform::CiscoIos,
            SshPlatform::HpComware,
            SshPlatform::ArubaAos,
        ] {
            let profile = commands(platform);
            assert!(!profile.is_empty());
            assert!(profile.iter().all(|command| !command.trim().is_empty()));
        }
    }

    #[test]
    fn truncation_preserves_utf8_boundaries() {
        assert_eq!(truncate_utf8("abc🙂def".to_string(), 5), "abc");
    }

    #[test]
    fn standard_ssh_port_uses_named_port_type() {
        assert_eq!(port_type(22), PortType::Ssh);
        assert_eq!(port_type(2222), PortType::new_tcp(2222));
    }
}

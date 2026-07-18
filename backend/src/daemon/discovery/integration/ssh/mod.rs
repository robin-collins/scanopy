//! Read-only SSH discovery integration.
//!
//! Adapted from the CodexNet collector design under AGPL-3.0 with permission from its author.
//! Commands are selected only from a platform-specific static allowlist; credentials and command
//! output are never logged.

use std::{
    future::Future,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

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
const PROBE_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_COMMAND_OUTPUT: usize = 1024 * 1024;
const MAX_STORED_DESCRIPTION: usize = 64 * 1024;
const MAX_HOSTNAME_LENGTH: usize = 253;

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

        let session = match connect_with_control(
            ctx.cancel,
            PROBE_TIMEOUT,
            connect(ctx.ip.to_string(), credential),
        )
        .await
        {
            Ok(session) => session,
            Err(ControlledConnectError::Cancelled) => return Err(failure("cancelled")),
            Err(ControlledConnectError::TimedOut) => {
                tracing::debug!(ip = %ctx.ip, "SSH credential probe timed out");
                return Err(failure(
                    "SSH connection, host verification, or authentication timed out",
                ));
            }
            Err(ControlledConnectError::Failed(error)) => {
                tracing::debug!(ip = %ctx.ip, error = %error, "SSH credential probe failed");
                return Err(failure(
                    "SSH connection, host verification, or authentication failed",
                ));
            }
        };

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
        let collected = collect_command_outputs(
            &mut session,
            handle.platform,
            ctx.ip,
            ctx.cancel,
            COMMAND_TIMEOUT,
        )
        .await;

        let _ = session
            .disconnect(Disconnect::ByApplication, "", "English")
            .await;
        let successful = collected?;
        enrich_host_data(handle.platform, &successful, host_data);
        Ok(())
    }
}

enum ControlledConnectError {
    Cancelled,
    TimedOut,
    Failed(Error),
}

async fn connect_with_control<F, T>(
    cancel: &tokio_util::sync::CancellationToken,
    deadline: Duration,
    connect_future: F,
) -> Result<T, ControlledConnectError>
where
    F: Future<Output = Result<T, Error>>,
{
    tokio::select! {
        _ = cancel.cancelled() => Err(ControlledConnectError::Cancelled),
        result = tokio::time::timeout(deadline, connect_future) => match result {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(error)) => Err(ControlledConnectError::Failed(error)),
            Err(_) => Err(ControlledConnectError::TimedOut),
        },
    }
}

async fn collect_command_outputs(
    session: &mut client::Handle<SshClientHandler>,
    platform: SshPlatform,
    ip: std::net::IpAddr,
    cancel: &tokio_util::sync::CancellationToken,
    command_timeout: Duration,
) -> Result<Vec<(&'static str, String)>, Error> {
    let mut successful = Vec::new();
    let mut successful_channels = 0;
    for command in commands(platform) {
        if cancel.is_cancelled() {
            return Err(anyhow!("discovery was cancelled"));
        }
        let command_result = tokio::select! {
            _ = cancel.cancelled() => return Err(anyhow!("discovery was cancelled")),
            result = tokio::time::timeout(command_timeout, execute_command(session, command)) => result,
        };
        match command_result {
            Ok(Ok(output)) => {
                successful_channels += 1;
                if !output.trim().is_empty() {
                    successful.push((*command, output));
                }
            }
            Ok(Err(error)) => {
                tracing::debug!(ip = %ip, command, error = %error, "Read-only SSH command failed");
            }
            Err(_) => {
                tracing::debug!(ip = %ip, command, "Read-only SSH command timed out");
            }
        }
    }
    if successful_channels == 0 {
        return Err(anyhow!("all read-only SSH commands failed or timed out"));
    }
    Ok(successful)
}

fn enrich_host_data(
    platform: SshPlatform,
    successful: &[(&'static str, String)],
    host_data: &mut HostData,
) {
    if platform == SshPlatform::Linux
        && let Some((_, hostname)) = successful
            .iter()
            .find(|(command, _)| *command == "hostname")
        && let Some(hostname) = normalize_hostname(hostname)
    {
        host_data.with_hostname_fallback(hostname.clone());
        host_data.with_sys_name(hostname);
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
}

fn normalize_hostname(output: &str) -> Option<String> {
    let hostname = output.lines().next()?.trim().trim_end_matches('.');
    if hostname.is_empty()
        || hostname.len() > MAX_HOSTNAME_LENGTH
        || !hostname.is_ascii()
        || !hostname.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
    {
        return None;
    }
    Some(hostname.to_string())
}

async fn connect(
    host: String,
    credential: &SshQueryCredential,
) -> Result<client::Handle<SshClientHandler>, Error> {
    if credential.host_key_policy == SshHostKeyPolicy::Strict {
        let known_hosts_file = credential
            .known_hosts_file
            .as_deref()
            .ok_or_else(|| anyhow!("strict SSH host-key policy has no known_hosts file"))?;
        if !Path::new(known_hosts_file).is_absolute() {
            return Err(anyhow!(
                "SSH known_hosts file is not absolute on the daemon platform"
            ));
        }
    }
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
    let mut received_bytes = 0usize;
    let mut exit_status = None;
    while let Some(message) = channel.wait().await {
        match message {
            ChannelMsg::Data { data } => {
                received_bytes = received_bytes.saturating_add(data.len());
                if received_bytes > MAX_COMMAND_OUTPUT {
                    return Err(anyhow!("SSH command output exceeded the configured limit"));
                }
                output.extend_from_slice(data.as_ref());
            }
            ChannelMsg::ExtendedData { data, .. } => {
                received_bytes = received_bytes.saturating_add(data.len());
                if received_bytes > MAX_COMMAND_OUTPUT {
                    return Err(anyhow!("SSH command output exceeded the configured limit"));
                }
            }
            ChannelMsg::ExitStatus {
                exit_status: status,
            } => exit_status = Some(status),
            _ => {}
        }
    }
    if let Some(status) = exit_status
        && status != 0
    {
        return Err(anyhow!("SSH command exited with status {status}"));
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
    use std::{net::Ipv4Addr, sync::Mutex as StdMutex};

    use keys::ssh_key::{getrandom::SysRng, rand_core::UnwrapErr};
    use russh::{Channel, ChannelId, server};
    use tokio::{net::TcpListener, task::JoinHandle};
    use tokio_util::sync::CancellationToken;

    use crate::server::hosts::r#impl::base::{Host, HostBase};

    const TEST_USERNAME: &str = "read-only-user";
    const TEST_PASSWORD: &str = "correct horse battery staple";

    fn random_private_key() -> keys::PrivateKey {
        let mut rng = UnwrapErr(SysRng);
        keys::PrivateKey::random(&mut rng, keys::ssh_key::Algorithm::Ed25519).unwrap()
    }

    #[derive(Clone, Copy)]
    enum CommandBehavior {
        Linux,
        Oversized,
        HangHostname,
        Rejected,
    }

    #[derive(Clone)]
    struct MockHandler {
        accepted_password: Option<String>,
        accepted_public_key: Option<keys::ssh_key::PublicKey>,
        behavior: CommandBehavior,
        commands: Arc<StdMutex<Vec<String>>>,
    }

    impl server::Handler for MockHandler {
        type Error = anyhow::Error;

        async fn auth_password(
            &mut self,
            user: &str,
            password: &str,
        ) -> Result<server::Auth, Self::Error> {
            if user == TEST_USERNAME && self.accepted_password.as_deref() == Some(password) {
                Ok(server::Auth::Accept)
            } else {
                Ok(server::Auth::reject())
            }
        }

        async fn auth_publickey(
            &mut self,
            user: &str,
            public_key: &keys::ssh_key::PublicKey,
        ) -> Result<server::Auth, Self::Error> {
            if user == TEST_USERNAME && self.accepted_public_key.as_ref() == Some(public_key) {
                Ok(server::Auth::Accept)
            } else {
                Ok(server::Auth::reject())
            }
        }

        async fn channel_open_session(
            &mut self,
            _channel: Channel<server::Msg>,
            reply: server::ChannelOpenHandle,
            _session: &mut server::Session,
        ) -> Result<(), Self::Error> {
            reply.accept().await;
            Ok(())
        }

        async fn exec_request(
            &mut self,
            channel: ChannelId,
            data: &[u8],
            session: &mut server::Session,
        ) -> Result<(), Self::Error> {
            let command = String::from_utf8_lossy(data).into_owned();
            self.commands.lock().unwrap().push(command.clone());
            session.channel_success(channel)?;

            if matches!(self.behavior, CommandBehavior::HangHostname) && command == "hostname" {
                return Ok(());
            }

            let output = match self.behavior {
                CommandBehavior::Oversized => vec![b'x'; MAX_COMMAND_OUTPUT + 1],
                CommandBehavior::Linux | CommandBehavior::HangHostname => linux_output(&command),
                CommandBehavior::Rejected => {
                    session.extended_data(channel, 1, b"sensitive denial detail".as_slice())?;
                    Vec::new()
                }
            };
            if !output.is_empty() {
                session.data(channel, output)?;
            }
            let status = if matches!(self.behavior, CommandBehavior::Rejected) {
                1
            } else {
                0
            };
            session.exit_status_request(channel, status)?;
            session.eof(channel)?;
            session.close(channel)?;
            Ok(())
        }
    }

    fn linux_output(command: &str) -> Vec<u8> {
        match command {
            "hostname" => b"mock-host\n".to_vec(),
            "uname -a" => b"Linux mock-host 6.12.0 test\n".to_vec(),
            "cat /etc/os-release" => b"NAME=Mock Linux\n".to_vec(),
            _ => Vec::new(),
        }
    }

    struct MockServer {
        port: u16,
        host_public_key: keys::ssh_key::PublicKey,
        commands: Arc<StdMutex<Vec<String>>>,
        task: JoinHandle<()>,
    }

    impl MockServer {
        async fn start(
            accepted_password: Option<&str>,
            accepted_public_key: Option<keys::ssh_key::PublicKey>,
            behavior: CommandBehavior,
        ) -> Self {
            let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let host_key = random_private_key();
            let host_public_key = host_key.public_key().clone();
            let config = Arc::new(server::Config {
                auth_rejection_time: Duration::ZERO,
                auth_rejection_time_initial: Some(Duration::ZERO),
                keys: vec![host_key],
                ..Default::default()
            });
            let commands = Arc::new(StdMutex::new(Vec::new()));
            let handler = MockHandler {
                accepted_password: accepted_password.map(str::to_string),
                accepted_public_key,
                behavior,
                commands: commands.clone(),
            };
            let task = tokio::spawn(async move {
                loop {
                    let Ok((socket, _)) = listener.accept().await else {
                        break;
                    };
                    let config = config.clone();
                    let handler = handler.clone();
                    tokio::spawn(async move {
                        if let Ok(session) = server::run_stream(config, socket, handler).await {
                            let _ = session.await;
                        }
                    });
                }
            });
            Self {
                port,
                host_public_key,
                commands,
                task,
            }
        }

        fn recorded_commands(&self) -> Vec<String> {
            self.commands.lock().unwrap().clone()
        }
    }

    impl Drop for MockServer {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    fn password_credential(
        port: u16,
        password: &str,
        host_key_policy: SshHostKeyPolicy,
        known_hosts_file: Option<String>,
    ) -> SshQueryCredential {
        SshQueryCredential {
            username: TEST_USERNAME.to_string(),
            authentication: SshAuthentication::Password {
                password: ResolvableSecret::Value {
                    value: password.to_string(),
                },
            },
            port,
            platform: SshPlatform::Linux,
            host_key_policy,
            known_hosts_file,
        }
    }

    fn private_key_credential(
        port: u16,
        private_key: String,
        passphrase: Option<&str>,
    ) -> SshQueryCredential {
        SshQueryCredential {
            username: TEST_USERNAME.to_string(),
            authentication: SshAuthentication::PrivateKey {
                private_key: ResolvableSecret::Value { value: private_key },
                passphrase: passphrase.map(|value| ResolvableSecret::Value {
                    value: value.to_string(),
                }),
            },
            port,
            platform: SshPlatform::Linux,
            host_key_policy: SshHostKeyPolicy::AcceptUnknown,
            known_hosts_file: None,
        }
    }

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

    #[tokio::test]
    async fn password_authentication_accepts_valid_and_rejects_invalid_passwords() {
        let server = MockServer::start(Some(TEST_PASSWORD), None, CommandBehavior::Linux).await;
        let valid = password_credential(
            server.port,
            TEST_PASSWORD,
            SshHostKeyPolicy::AcceptUnknown,
            None,
        );
        let session = connect(Ipv4Addr::LOCALHOST.to_string(), &valid)
            .await
            .expect("valid password should authenticate");
        session
            .disconnect(Disconnect::ByApplication, "", "English")
            .await
            .unwrap();

        let invalid_password = "password-that-must-never-leak";
        let invalid = password_credential(
            server.port,
            invalid_password,
            SshHostKeyPolicy::AcceptUnknown,
            None,
        );
        let error = match connect(Ipv4Addr::LOCALHOST.to_string(), &invalid).await {
            Ok(_) => panic!("invalid password must be rejected"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("authentication rejected"));
        assert!(!error.contains(invalid_password));
        let debug = format!("{invalid:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains(invalid_password));
    }

    #[tokio::test]
    async fn controlled_connect_honors_cancellation_timeout_and_failures() {
        let cancel = CancellationToken::new();
        cancel.cancel();
        let cancelled = connect_with_control(&cancel, Duration::from_secs(5), async {
            std::future::pending::<Result<(), Error>>().await
        })
        .await;
        assert!(matches!(cancelled, Err(ControlledConnectError::Cancelled)));

        let timeout_cancel = CancellationToken::new();
        let timed_out = connect_with_control(
            &timeout_cancel,
            Duration::from_millis(10),
            std::future::pending::<Result<(), Error>>(),
        )
        .await;
        assert!(matches!(timed_out, Err(ControlledConnectError::TimedOut)));

        let failed = connect_with_control(&timeout_cancel, Duration::from_secs(1), async {
            Err::<(), Error>(anyhow!("safe test failure"))
        })
        .await;
        assert!(matches!(failed, Err(ControlledConnectError::Failed(_))));
    }

    #[tokio::test]
    async fn strict_host_key_policy_accepts_known_key_and_rejects_changed_key() {
        let server = MockServer::start(Some(TEST_PASSWORD), None, CommandBehavior::Linux).await;
        let directory = tempfile::tempdir().unwrap();
        let known_hosts = directory.path().join("known_hosts");
        keys::known_hosts::learn_known_hosts_path(
            &Ipv4Addr::LOCALHOST.to_string(),
            server.port,
            &server.host_public_key,
            &known_hosts,
        )
        .unwrap();
        let accepted = password_credential(
            server.port,
            TEST_PASSWORD,
            SshHostKeyPolicy::Strict,
            Some(known_hosts.to_string_lossy().into_owned()),
        );
        let session = connect(Ipv4Addr::LOCALHOST.to_string(), &accepted)
            .await
            .expect("known host key should be accepted");
        session
            .disconnect(Disconnect::ByApplication, "", "English")
            .await
            .unwrap();

        let changed_hosts = directory.path().join("changed_known_hosts");
        let other_key = random_private_key();
        keys::known_hosts::learn_known_hosts_path(
            &Ipv4Addr::LOCALHOST.to_string(),
            server.port,
            other_key.public_key(),
            &changed_hosts,
        )
        .unwrap();
        let rejected = password_credential(
            server.port,
            TEST_PASSWORD,
            SshHostKeyPolicy::Strict,
            Some(changed_hosts.to_string_lossy().into_owned()),
        );
        assert!(
            connect(Ipv4Addr::LOCALHOST.to_string(), &rejected)
                .await
                .is_err(),
            "changed host key must be rejected"
        );
    }

    #[tokio::test]
    async fn encrypted_private_key_authentication_uses_passphrase() {
        let passphrase = "test-key-passphrase";
        let private_key = random_private_key();
        let public_key = private_key.public_key().clone();
        let mut rng = UnwrapErr(SysRng);
        let encrypted_key = private_key.encrypt(&mut rng, passphrase).unwrap();
        let encoded = encrypted_key
            .to_openssh(keys::ssh_key::LineEnding::LF)
            .unwrap()
            .to_string();
        let server = MockServer::start(None, Some(public_key), CommandBehavior::Linux).await;
        let credential = private_key_credential(server.port, encoded, Some(passphrase));

        let session = connect(Ipv4Addr::LOCALHOST.to_string(), &credential)
            .await
            .expect("encrypted private key should authenticate with its passphrase");
        session
            .disconnect(Disconnect::ByApplication, "", "English")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn command_output_is_bounded() {
        let server = MockServer::start(Some(TEST_PASSWORD), None, CommandBehavior::Oversized).await;
        let credential = password_credential(
            server.port,
            TEST_PASSWORD,
            SshHostKeyPolicy::AcceptUnknown,
            None,
        );
        let mut session = connect(Ipv4Addr::LOCALHOST.to_string(), &credential)
            .await
            .unwrap();
        let error = execute_command(&mut session, "hostname")
            .await
            .expect_err("oversized output must fail");
        assert!(error.to_string().contains("output exceeded"));
    }

    #[tokio::test]
    async fn command_collection_honors_timeout_and_cancellation() {
        let server =
            MockServer::start(Some(TEST_PASSWORD), None, CommandBehavior::HangHostname).await;
        let credential = password_credential(
            server.port,
            TEST_PASSWORD,
            SshHostKeyPolicy::AcceptUnknown,
            None,
        );
        let mut cancelled_session = connect(Ipv4Addr::LOCALHOST.to_string(), &credential)
            .await
            .unwrap();
        let cancel = CancellationToken::new();
        let cancel_trigger = cancel.clone();
        let recorded_commands = server.commands.clone();
        tokio::spawn(async move {
            loop {
                if !recorded_commands.lock().unwrap().is_empty() {
                    cancel_trigger.cancel();
                    return;
                }
                tokio::task::yield_now().await;
            }
        });
        let started = std::time::Instant::now();
        let error = collect_command_outputs(
            &mut cancelled_session,
            SshPlatform::Linux,
            Ipv4Addr::LOCALHOST.into(),
            &cancel,
            Duration::from_secs(5),
        )
        .await
        .expect_err("cancellation must interrupt an already-running command");
        assert_eq!(error.to_string(), "discovery was cancelled");
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "cancellation should not wait for the command timeout"
        );

        let mut timed_session = connect(Ipv4Addr::LOCALHOST.to_string(), &credential)
            .await
            .unwrap();
        let timeout_cancel = CancellationToken::new();
        let outputs = collect_command_outputs(
            &mut timed_session,
            SshPlatform::Linux,
            Ipv4Addr::LOCALHOST.into(),
            &timeout_cancel,
            Duration::from_millis(50),
        )
        .await
        .expect("a timed-out command should not abort the remaining allowlist");
        assert!(!outputs.iter().any(|(command, _)| *command == "hostname"));
        assert!(outputs.iter().any(|(command, _)| *command == "uname -a"));

        let before_cancel = server.recorded_commands().len();
        timeout_cancel.cancel();
        let error = collect_command_outputs(
            &mut timed_session,
            SshPlatform::Linux,
            Ipv4Addr::LOCALHOST.into(),
            &timeout_cancel,
            Duration::from_millis(50),
        )
        .await
        .expect_err("cancelled collection must stop before another command");
        assert_eq!(error.to_string(), "discovery was cancelled");
        assert_eq!(server.recorded_commands().len(), before_cancel);
    }

    #[tokio::test]
    async fn command_collection_rejects_when_every_command_fails() {
        let server = MockServer::start(Some(TEST_PASSWORD), None, CommandBehavior::Rejected).await;
        let credential = password_credential(
            server.port,
            TEST_PASSWORD,
            SshHostKeyPolicy::AcceptUnknown,
            None,
        );
        let mut session = connect(Ipv4Addr::LOCALHOST.to_string(), &credential)
            .await
            .unwrap();
        let error = collect_command_outputs(
            &mut session,
            SshPlatform::Linux,
            Ipv4Addr::LOCALHOST.into(),
            &CancellationToken::new(),
            Duration::from_secs(1),
        )
        .await
        .expect_err("total command failure must not look successful");
        assert!(
            error
                .to_string()
                .contains("all read-only SSH commands failed")
        );
        assert!(!error.to_string().contains("sensitive denial detail"));
    }

    #[test]
    fn linux_outputs_enrich_hostname_system_name_and_description() {
        let mut host_data = HostData::new(
            Host::new(HostBase::default()),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        let outputs = vec![
            ("hostname", " mock-host\n".to_string()),
            ("uname -a", "Linux mock-host 6.12.0 test\n".to_string()),
            ("cat /etc/os-release", "NAME=Mock Linux\n".to_string()),
        ];

        enrich_host_data(SshPlatform::Linux, &outputs, &mut host_data);

        assert_eq!(host_data.host.base.hostname.as_deref(), Some("mock-host"));
        assert_eq!(host_data.host.base.sys_name.as_deref(), Some("mock-host"));
        let description = host_data.host.base.sys_descr.unwrap();
        assert!(description.contains("$ uname -a\nLinux mock-host"));
        assert!(description.contains("$ cat /etc/os-release\nNAME=Mock Linux"));
    }

    #[test]
    fn hostname_normalization_rejects_unbounded_or_invalid_output() {
        assert_eq!(
            normalize_hostname("host-01.example.test\n"),
            Some("host-01.example.test".into())
        );
        assert_eq!(
            normalize_hostname("host.example.test.\n"),
            Some("host.example.test".into())
        );
        assert_eq!(normalize_hostname("-invalid.example.test\n"), None);
        assert_eq!(normalize_hostname("host name\n"), None);
        assert_eq!(
            normalize_hostname(&"a".repeat(MAX_HOSTNAME_LENGTH + 1)),
            None
        );
    }
}

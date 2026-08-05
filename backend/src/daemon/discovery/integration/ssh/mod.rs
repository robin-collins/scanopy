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
    hosts::r#impl::os::HostOsGroup,
    ports::r#impl::base::PortType,
    services::r#impl::patterns::ClientProbe,
};

use super::{
    DiscoveryIntegration, IntegrationContext, IntegrationFailure, ProbeContext, ProbeFailure,
    ProbeSuccess,
};
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
    "uptime",
    "nproc",
    "free -h",
    "df -h",
    "ss -tuln",
    "cat /sys/class/dmi/id/sys_vendor",
    "cat /sys/class/dmi/id/product_name",
    "cat /sys/class/dmi/id/product_serial",
];

const FREEBSD_MANUFACTURER_COMMAND: &str = "kenv -q smbios.system.maker";
const FREEBSD_MODEL_COMMAND: &str = "kenv -q smbios.system.product";
const FREEBSD_SERIAL_COMMAND: &str = "kenv -q smbios.system.serial";

/// Covers FreeBSD itself as well as FreeBSD-derived appliances (e.g.
/// OPNsense) that expose an interactive SSH shell — `kenv` reads the same
/// SMBIOS fields Linux gets from `/sys/class/dmi`.
const FREEBSD_COMMANDS: &[&str] = &[
    "hostname",
    "uname -a",
    "freebsd-version -u",
    "ifconfig -a",
    "netstat -rn",
    "uptime",
    "sysctl -n hw.ncpu",
    "sysctl -n hw.physmem",
    "df -h",
    "sockstat -46l",
    FREEBSD_MANUFACTURER_COMMAND,
    FREEBSD_MODEL_COMMAND,
    FREEBSD_SERIAL_COMMAND,
];

const WINDOWS_OS_CAPTION_COMMAND: &str = r#"powershell -NoProfile -NonInteractive -Command "(Get-CimInstance Win32_OperatingSystem).Caption""#;
const WINDOWS_OS_VERSION_COMMAND: &str = r#"powershell -NoProfile -NonInteractive -Command "(Get-CimInstance Win32_OperatingSystem).Version""#;
const WINDOWS_MANUFACTURER_COMMAND: &str = r#"powershell -NoProfile -NonInteractive -Command "(Get-CimInstance Win32_ComputerSystem).Manufacturer""#;
const WINDOWS_MODEL_COMMAND: &str = r#"powershell -NoProfile -NonInteractive -Command "(Get-CimInstance Win32_ComputerSystem).Model""#;
const WINDOWS_SERIAL_COMMAND: &str =
    r#"powershell -NoProfile -NonInteractive -Command "(Get-CimInstance Win32_BIOS).SerialNumber""#;
const WINDOWS_CPU_COUNT_COMMAND: &str = r#"powershell -NoProfile -NonInteractive -Command "(Get-CimInstance Win32_ComputerSystem).NumberOfLogicalProcessors""#;
const WINDOWS_MEMORY_COMMAND: &str = r#"powershell -NoProfile -NonInteractive -Command "(Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory""#;

/// Windows' OpenSSH server defaults the exec shell to `cmd.exe` unless the
/// host has set the `DefaultShell` registry value, so every command here is
/// explicitly dispatched through `powershell.exe` rather than relying on the
/// server's default shell.
const WINDOWS_COMMANDS: &[&str] = &[
    "hostname",
    WINDOWS_OS_CAPTION_COMMAND,
    WINDOWS_OS_VERSION_COMMAND,
    WINDOWS_MANUFACTURER_COMMAND,
    WINDOWS_MODEL_COMMAND,
    WINDOWS_SERIAL_COMMAND,
    WINDOWS_CPU_COUNT_COMMAND,
    WINDOWS_MEMORY_COMMAND,
    "ipconfig /all",
    "netstat -ano",
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

/// Refined-collection follow-up commands, selected by detected `HostOsGroup`
/// rather than `SshPlatform` — same static-allowlist discipline as
/// `commands()`, just keyed on a different classification.
const LINUX_DEBIAN_REFINED_COMMANDS: &[&str] = &["dpkg -l | wc -l"];

fn commands(platform: SshPlatform) -> &'static [&'static str] {
    match platform {
        SshPlatform::Linux => LINUX_COMMANDS,
        SshPlatform::FreeBsd => FREEBSD_COMMANDS,
        SshPlatform::Windows => WINDOWS_COMMANDS,
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
        // Linux now runs 14 commands (up from 6); keep the same ~1.5x safety
        // margin over the worst case of every command hitting COMMAND_TIMEOUT.
        Duration::from_secs(420)
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
            _ => return Err(ProbeFailure::malformed("expected SSH credential")),
        };

        if ctx.cancel.is_cancelled() {
            return Err(ProbeFailure::cancelled());
        }

        let session = match connect_with_control(
            ctx.cancel,
            PROBE_TIMEOUT,
            connect(ctx.ip.to_string(), credential),
        )
        .await
        {
            Ok(session) => session,
            Err(ControlledConnectError::Cancelled) => return Err(ProbeFailure::cancelled()),
            Err(ControlledConnectError::TimedOut) => {
                tracing::debug!(ip = %ctx.ip, "SSH credential probe timed out");
                return Err(ProbeFailure::timed_out(
                    "SSH connection, host verification, or authentication timed out",
                ));
            }
            Err(ControlledConnectError::Failed(error)) => {
                tracing::debug!(ip = %ctx.ip, error = %error, "SSH credential probe failed");
                return Err(ProbeFailure::rejected(
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
    ) -> Result<(), IntegrationFailure> {
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

        let mut successful = collected?;

        // Refined collection: once the OS group is known from this same
        // scan's baseline output, run one extra group-specific command
        // before disconnecting. This is deliberately scoped to a single
        // additional command per group, not an open-ended follow-up list.
        if handle.platform == SshPlatform::Linux
            && detect_linux_os_group(&successful) == Some(HostOsGroup::LinuxDebian)
            && !ctx.cancel.is_cancelled()
            && let Ok(Ok(output)) = tokio::time::timeout(
                COMMAND_TIMEOUT,
                execute_command(&mut session, LINUX_DEBIAN_REFINED_COMMANDS[0]),
            )
            .await
            && !output.trim().is_empty()
        {
            successful.push((LINUX_DEBIAN_REFINED_COMMANDS[0], output));
        }

        let _ = session
            .disconnect(Disconnect::ByApplication, "", "English")
            .await;
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

/// Read a single-line DMI sysfs value from a command's output, trimmed and
/// rejected if empty or if the read failed silently (some kernels return the
/// literal string "None" for absent DMI fields rather than an empty file).
fn dmi_value(successful: &[(&'static str, String)], command: &str) -> Option<String> {
    let (_, output) = successful.iter().find(|(c, _)| *c == command)?;
    let value = output.lines().next()?.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("none") {
        return None;
    }
    Some(value.to_string())
}

/// Read `/etc/os-release`'s `ID`/`ID_LIKE` fields to classify Debian-derived
/// distros. Only called for `SshPlatform::Linux` — the network-device
/// platforms (Cisco IOS/HP Comware/ArubaOS) aren't reliably classifiable as
/// router vs. switch from their SSH output alone, so they're left for manual
/// assignment rather than guessed.
fn detect_linux_os_group(successful: &[(&'static str, String)]) -> Option<HostOsGroup> {
    let (_, os_release) = successful
        .iter()
        .find(|(command, _)| *command == "cat /etc/os-release")?;
    let is_debian_like = os_release.lines().any(|line| {
        let Some((key, value)) = line.split_once('=') else {
            return false;
        };
        matches!(key, "ID" | "ID_LIKE") && value.to_lowercase().contains("debian")
    });
    Some(if is_debian_like {
        HostOsGroup::LinuxDebian
    } else {
        HostOsGroup::Linux
    })
}

fn enrich_host_data(
    platform: SshPlatform,
    successful: &[(&'static str, String)],
    host_data: &mut HostData,
) {
    if matches!(
        platform,
        SshPlatform::Linux | SshPlatform::FreeBsd | SshPlatform::Windows
    ) && let Some((_, hostname)) = successful
        .iter()
        .find(|(command, _)| *command == "hostname")
        && let Some(hostname) = normalize_hostname(hostname)
    {
        host_data.with_hostname_fallback(hostname.clone());
        host_data.with_sys_name(hostname);
    }

    if platform == SshPlatform::Linux {
        if let Some(group) = detect_linux_os_group(successful) {
            host_data.with_os_group(group);
        }
        if let Some(vendor) = dmi_value(successful, "cat /sys/class/dmi/id/sys_vendor") {
            host_data.with_manufacturer(vendor);
        }
        if let Some(product) = dmi_value(successful, "cat /sys/class/dmi/id/product_name") {
            host_data.with_model(product);
        }
        if let Some(serial) = dmi_value(successful, "cat /sys/class/dmi/id/product_serial") {
            host_data.with_serial_number(serial);
        }
    }

    if platform == SshPlatform::FreeBsd {
        if let Some(vendor) = dmi_value(successful, FREEBSD_MANUFACTURER_COMMAND) {
            host_data.with_manufacturer(vendor);
        }
        if let Some(product) = dmi_value(successful, FREEBSD_MODEL_COMMAND) {
            host_data.with_model(product);
        }
        if let Some(serial) = dmi_value(successful, FREEBSD_SERIAL_COMMAND) {
            host_data.with_serial_number(serial);
        }
    }

    // Windows is unambiguous from the credential's platform selection itself —
    // unlike Linux/FreeBSD there's no output to classify further.
    if platform == SshPlatform::Windows {
        host_data.with_os_group(HostOsGroup::Windows);
        if let Some(vendor) = dmi_value(successful, WINDOWS_MANUFACTURER_COMMAND) {
            host_data.with_manufacturer(vendor);
        }
        if let Some(product) = dmi_value(successful, WINDOWS_MODEL_COMMAND) {
            host_data.with_model(product);
        }
        if let Some(serial) = dmi_value(successful, WINDOWS_SERIAL_COMMAND) {
            host_data.with_serial_number(serial);
        }
    }

    let description = successful
        .iter()
        .filter(|(command, _)| {
            matches!(
                *command,
                "uname -a"
                    | "cat /etc/os-release"
                    | "freebsd-version -u"
                    | "show version"
                    | "display version"
                    | "show system"
                    | "uptime"
                    | "nproc"
                    | "free -h"
                    | "df -h"
                    | "ss -tuln"
                    | "sockstat -46l"
                    | "ipconfig /all"
                    | "dpkg -l | wc -l"
            ) || matches!(
                *command,
                WINDOWS_OS_CAPTION_COMMAND | WINDOWS_OS_VERSION_COMMAND
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
            || commands(SshPlatform::FreeBsd).contains(&command)
            || commands(SshPlatform::Windows).contains(&command)
            || commands(SshPlatform::CiscoIos).contains(&command)
            || commands(SshPlatform::HpComware).contains(&command)
            || commands(SshPlatform::ArubaAos).contains(&command)
            || LINUX_DEBIAN_REFINED_COMMANDS.contains(&command)
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
            "cat /sys/class/dmi/id/sys_vendor" => b"Mock Vendor\n".to_vec(),
            "cat /sys/class/dmi/id/product_name" => b"Mock Server X1\n".to_vec(),
            "cat /sys/class/dmi/id/product_serial" => b"None\n".to_vec(),
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
            SshPlatform::FreeBsd,
            SshPlatform::Windows,
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
    fn freebsd_outputs_enrich_hostname_and_smbios_identity() {
        let mut host_data = HostData::new(
            Host::new(HostBase::default()),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        let outputs = vec![
            ("hostname", "mock-bsd\n".to_string()),
            ("uname -a", "FreeBSD mock-bsd 14.1-RELEASE\n".to_string()),
            (FREEBSD_MANUFACTURER_COMMAND, "Mock Vendor\n".to_string()),
            (FREEBSD_MODEL_COMMAND, "Mock Appliance\n".to_string()),
            (FREEBSD_SERIAL_COMMAND, "None\n".to_string()),
        ];

        enrich_host_data(SshPlatform::FreeBsd, &outputs, &mut host_data);

        assert_eq!(host_data.host.base.hostname.as_deref(), Some("mock-bsd"));
        assert_eq!(host_data.host.base.sys_name.as_deref(), Some("mock-bsd"));
        assert_eq!(host_data.host.base.os_group, None);
        assert_eq!(
            host_data.host.base.manufacturer.as_deref(),
            Some("Mock Vendor")
        );
        assert_eq!(host_data.host.base.model.as_deref(), Some("Mock Appliance"));
        assert_eq!(host_data.host.base.serial_number.as_deref(), None);
        let description = host_data.host.base.sys_descr.unwrap();
        assert!(description.contains("$ uname -a\nFreeBSD mock-bsd"));
    }

    #[test]
    fn windows_outputs_enrich_hostname_os_group_and_identity() {
        let mut host_data = HostData::new(
            Host::new(HostBase::default()),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        let outputs = vec![
            ("hostname", "MOCK-WIN\n".to_string()),
            (
                WINDOWS_OS_CAPTION_COMMAND,
                "Microsoft Windows Server 2022 Standard\n".to_string(),
            ),
            (WINDOWS_MANUFACTURER_COMMAND, "Mock PC Vendor\n".to_string()),
            (WINDOWS_MODEL_COMMAND, "Mock Model X\n".to_string()),
            (WINDOWS_SERIAL_COMMAND, "MOCK-SERIAL-123\n".to_string()),
        ];

        enrich_host_data(SshPlatform::Windows, &outputs, &mut host_data);

        assert_eq!(host_data.host.base.hostname.as_deref(), Some("MOCK-WIN"));
        assert_eq!(host_data.host.base.sys_name.as_deref(), Some("MOCK-WIN"));
        assert_eq!(host_data.host.base.os_group, Some(HostOsGroup::Windows));
        assert_eq!(
            host_data.host.base.manufacturer.as_deref(),
            Some("Mock PC Vendor")
        );
        assert_eq!(host_data.host.base.model.as_deref(), Some("Mock Model X"));
        assert_eq!(
            host_data.host.base.serial_number.as_deref(),
            Some("MOCK-SERIAL-123")
        );
        let description = host_data.host.base.sys_descr.unwrap();
        assert!(description.contains("$ ") && description.contains("Windows Server 2022"));
    }

    #[test]
    fn linux_outputs_enrich_manufacturer_model_and_serial() {
        let mut host_data = HostData::new(
            Host::new(HostBase::default()),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        let outputs = vec![
            (
                "cat /sys/class/dmi/id/sys_vendor",
                "Mock Vendor\n".to_string(),
            ),
            (
                "cat /sys/class/dmi/id/product_name",
                "Mock Server X1\n".to_string(),
            ),
            // Kernels expose "None" (not an empty file) for absent DMI fields.
            ("cat /sys/class/dmi/id/product_serial", "None\n".to_string()),
            ("uptime", " 10:00:00 up 1 day\n".to_string()),
            ("nproc", "4\n".to_string()),
            ("free -h", "Mem: 16Gi\n".to_string()),
            ("df -h", "/dev/sda1 20G\n".to_string()),
            ("ss -tuln", "LISTEN 0.0.0.0:22\n".to_string()),
        ];

        enrich_host_data(SshPlatform::Linux, &outputs, &mut host_data);

        assert_eq!(
            host_data.host.base.manufacturer.as_deref(),
            Some("Mock Vendor")
        );
        assert_eq!(host_data.host.base.model.as_deref(), Some("Mock Server X1"));
        assert_eq!(host_data.host.base.serial_number.as_deref(), None);
        let description = host_data.host.base.sys_descr.unwrap();
        assert!(description.contains("$ uptime\n"));
        assert!(description.contains("$ nproc\n4"));
        assert!(description.contains("$ free -h\nMem: 16Gi"));
        assert!(description.contains("$ df -h\n/dev/sda1 20G"));
        assert!(description.contains("$ ss -tuln\nLISTEN"));
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

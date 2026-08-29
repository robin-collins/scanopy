//! WinRM discovery integration for Windows Server/Desktop hosts.
//!
//! This runs a short PowerShell script over WinRM's classic "cmd" shell
//! transport (the same WS-Management `Create`/`Command`/`Receive`/`Signal`/
//! `Delete` operations `winrs` uses), NOT the PowerShell Remoting Protocol
//! (PSRP/`Enter-PSSession`) — PSRP layers a binary-fragmented, CLIXML object
//! pipeline on top of the same transport and is out of scope here. The
//! script ends with `ConvertTo-Json`, so we get structured text back without
//! implementing PSRP's object serialization.
//!
//! Authentication is NTLMv2 (see `ntlm.rs`) with **no message-level
//! signing/sealing**. WinRM's default `AllowUnencrypted=false` server policy
//! rejects unsigned/unsealed requests over plain HTTP, so a target must
//! either serve HTTPS (`use_tls`, transport encryption satisfies the
//! requirement) or have `AllowUnencrypted` explicitly enabled for HTTP
//! (`winrm set winrm/config/service @{AllowUnencrypted="true"}`). This is
//! the same constraint most non-Kerberos WinRM clients (e.g. Ansible's
//! `winrm` connection plugin) document for NTLM-over-HTTP.
//!
//! NTLM is connection-oriented: the 3-leg handshake authenticates the
//! underlying TCP connection, and every subsequent WS-Man call in the same
//! shell session must reuse that exact connection. We rely on a dedicated
//! `reqwest::Client` per session with `pool_max_idle_per_host(1)` and never
//! issue concurrent requests on it — reqwest/hyper reuses the sole pooled
//! connection reliably under that condition. If the pool ever hands back a
//! fresh (unauthenticated) connection, the affected call gets a 401 and we
//! surface a clear, distinguishable error rather than silently misbehaving.

mod ntlm;
mod soap;

use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use anyhow::{Error, anyhow};
use async_trait::async_trait;
use base64::Engine;
use reqwest::StatusCode;

use crate::server::{
    credentials::r#impl::mapping::{
        CredentialQueryPayload, CredentialQueryPayloadDiscriminants, ResolvableSecret,
        WindowsDomainAccountQueryCredential, WindowsLocalAccountQueryCredential,
    },
    ports::r#impl::base::PortType,
    services::r#impl::patterns::ClientProbe,
};

use super::{
    Checkpoint, Completeness, DiscoveryIntegration, IntegrationContext, IntegrationFailure,
    InterfaceViewScope, ProbeContext, ProbeFailure, ProbeSuccess,
};
use crate::daemon::discovery::service::ops::HostData;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const CALL_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_RECEIVE_POLLS: u32 = 60;
const RECEIVE_POLL_DELAY: Duration = Duration::from_millis(500);
const MAX_COMMAND_OUTPUT: usize = 1024 * 1024;
const MAX_STORED_DESCRIPTION: usize = 64 * 1024;

/// One inventory pass: OS identity, hardware identity, and domain membership
/// via `Get-CimInstance`, emitted as compact JSON so we avoid implementing
/// PSRP's object pipeline (see module docs).
const COLLECTION_SCRIPT: &str = r#"$cs=Get-CimInstance Win32_ComputerSystem;$os=Get-CimInstance Win32_OperatingSystem;$bios=Get-CimInstance Win32_BIOS;([ordered]@{ComputerName=$cs.Name;Manufacturer=$cs.Manufacturer;Model=$cs.Model;Domain=$cs.Domain;PartOfDomain=$cs.PartOfDomain;SerialNumber=$bios.SerialNumber;OsCaption=$os.Caption;OsVersion=$os.Version;OsBuildNumber=$os.BuildNumber;OsArchitecture=$os.OSArchitecture})|ConvertTo-Json -Compress"#;

#[derive(Debug, Clone, serde::Deserialize)]
struct CollectionResult {
    #[serde(rename = "ComputerName")]
    computer_name: Option<String>,
    #[serde(rename = "Manufacturer")]
    manufacturer: Option<String>,
    #[serde(rename = "Model")]
    model: Option<String>,
    #[serde(rename = "Domain")]
    domain: Option<String>,
    #[serde(rename = "PartOfDomain")]
    part_of_domain: Option<bool>,
    #[serde(rename = "SerialNumber")]
    serial_number: Option<String>,
    #[serde(rename = "OsCaption")]
    os_caption: Option<String>,
    #[serde(rename = "OsVersion")]
    os_version: Option<String>,
    #[serde(rename = "OsBuildNumber")]
    os_build_number: Option<String>,
    #[serde(rename = "OsArchitecture")]
    os_architecture: Option<String>,
}

/// Shared connection parameters, independent of local-vs-domain auth.
struct WinRmTarget {
    port: u16,
    use_tls: bool,
}

fn endpoint(ip: IpAddr, target: &WinRmTarget) -> String {
    let scheme = if target.use_tls { "https" } else { "http" };
    format!("{scheme}://{ip}:{}/wsman", target.port)
}

fn build_client(
    ip: IpAddr,
    target: &WinRmTarget,
    accept_invalid_certs: bool,
) -> Result<reqwest::Client, Error> {
    let addr = SocketAddr::new(ip, target.port);
    let host = ip.to_string();
    let mut builder = reqwest::Client::builder()
        .http1_only()
        .pool_max_idle_per_host(1)
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(CALL_TIMEOUT)
        .resolve(&host, addr);
    if target.use_tls {
        builder = builder.danger_accept_invalid_certs(accept_invalid_certs);
    }
    builder.build().map_err(Error::from)
}

/// Perform the NTLM handshake by round-tripping the given SOAP body twice —
/// once per handshake leg — on `client`. Returns the authenticated response
/// body from the second leg (the actual WS-Man response).
async fn ntlm_authenticated_post(
    client: &reqwest::Client,
    url: &str,
    body: String,
    domain: &str,
    username: &str,
    password: &str,
) -> Result<String, Error> {
    let type1 = ntlm::negotiate_message();
    let leg1 = client
        .post(url)
        .header("Content-Type", "application/soap+xml;charset=UTF-8")
        .header(
            "Authorization",
            format!(
                "NTLM {}",
                base64::engine::general_purpose::STANDARD.encode(type1)
            ),
        )
        .body(body.clone())
        .send()
        .await?;
    if leg1.status() != StatusCode::UNAUTHORIZED {
        return Err(anyhow!(
            "WinRM did not challenge for NTLM (status {}) — target may not support NTLM auth",
            leg1.status()
        ));
    }
    let challenge_b64 = leg1
        .headers()
        .get_all("www-authenticate")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find_map(|v| v.strip_prefix("NTLM "))
        .ok_or_else(|| anyhow!("WinRM did not return an NTLM challenge"))?
        .to_string();
    let type2_bytes = base64::engine::general_purpose::STANDARD
        .decode(challenge_b64.trim())
        .map_err(|_| anyhow!("malformed NTLM challenge"))?;
    let challenge = ntlm::parse_challenge(&type2_bytes).map_err(|e| anyhow!(e.to_string()))?;
    let type3 = ntlm::authenticate_message(&ntlm::AuthenticateInputs {
        challenge: &challenge,
        domain,
        username,
        password,
    });

    let leg2 = client
        .post(url)
        .header("Content-Type", "application/soap+xml;charset=UTF-8")
        .header(
            "Authorization",
            format!(
                "NTLM {}",
                base64::engine::general_purpose::STANDARD.encode(type3)
            ),
        )
        .body(body)
        .send()
        .await?;
    if leg2.status() == StatusCode::UNAUTHORIZED {
        return Err(anyhow!("WinRM rejected NTLM authentication"));
    }
    if !leg2.status().is_success() {
        return Err(anyhow!("WinRM returned status {}", leg2.status()));
    }
    Ok(leg2.text().await?)
}

/// A follow-up call on an already-authenticated connection. A 401 here means
/// the connection pool handed us a fresh, unauthenticated connection — NTLM
/// affinity broke — which we surface distinctly rather than as a generic
/// auth failure, since it points at a transport issue, not bad credentials.
async fn authenticated_post(
    client: &reqwest::Client,
    url: &str,
    body: String,
) -> Result<String, Error> {
    let response = client
        .post(url)
        .header("Content-Type", "application/soap+xml;charset=UTF-8")
        .body(body)
        .send()
        .await?;
    if response.status() == StatusCode::UNAUTHORIZED {
        return Err(anyhow!(
            "WinRM connection lost its NTLM authentication mid-session (connection was not reused)"
        ));
    }
    if !response.status().is_success() {
        return Err(anyhow!("WinRM returned status {}", response.status()));
    }
    Ok(response.text().await?)
}

struct WinRmProbeHandle {
    client: reqwest::Client,
    endpoint: String,
    shell_id: String,
}

async fn open_shell(
    ip: IpAddr,
    target: &WinRmTarget,
    accept_invalid_certs: bool,
    domain: &str,
    username: &str,
    password: &str,
) -> Result<WinRmProbeHandle, Error> {
    let client = build_client(ip, target, accept_invalid_certs)?;
    let url = endpoint(ip, target);
    let body = soap::create_shell(&url);
    let response = ntlm_authenticated_post(&client, &url, body, domain, username, password).await?;
    let shell_id = soap::parse_element_text(&response, "ShellId")
        .ok_or_else(|| anyhow!("WinRM Create response did not contain a ShellId"))?;
    Ok(WinRmProbeHandle {
        client,
        endpoint: url,
        shell_id,
    })
}

async fn run_collection_script(handle: &WinRmProbeHandle) -> Result<String, Error> {
    let encoded_command = base64::engine::general_purpose::STANDARD.encode(
        COLLECTION_SCRIPT
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<u8>>(),
    );
    let command_body = soap::run_command(
        &handle.endpoint,
        &handle.shell_id,
        "powershell.exe",
        &[
            "-NoProfile",
            "-NonInteractive",
            "-EncodedCommand",
            &encoded_command,
        ],
    );
    let response = authenticated_post(&handle.client, &handle.endpoint, command_body).await?;
    let command_id = soap::parse_element_text(&response, "CommandId")
        .ok_or_else(|| anyhow!("WinRM Command response did not contain a CommandId"))?;

    let mut stdout = Vec::new();
    let mut done = false;
    for _ in 0..MAX_RECEIVE_POLLS {
        let receive_body = soap::receive(&handle.endpoint, &handle.shell_id, &command_id);
        let receive_response =
            authenticated_post(&handle.client, &handle.endpoint, receive_body).await?;
        let parsed = soap::parse_receive_response(&receive_response);
        stdout.extend_from_slice(&parsed.stdout);
        if stdout.len() > MAX_COMMAND_OUTPUT {
            let _ = signal_and_delete(handle, &command_id).await;
            return Err(anyhow!(
                "WinRM command output exceeded the configured limit"
            ));
        }
        if parsed.done {
            done = true;
            break;
        }
        tokio::time::sleep(RECEIVE_POLL_DELAY).await;
    }
    signal_and_delete(handle, &command_id).await;
    if !done {
        return Err(anyhow!(
            "WinRM command did not complete within the poll budget"
        ));
    }
    Ok(String::from_utf8_lossy(&stdout).into_owned())
}

/// Best-effort cleanup — mirrors the SSH integration's `let _ =` disconnect:
/// a failure here shouldn't fail collection when we already have output.
async fn signal_and_delete(handle: &WinRmProbeHandle, command_id: &str) {
    let signal_body = soap::signal_terminate(&handle.endpoint, &handle.shell_id, command_id);
    let _ = authenticated_post(&handle.client, &handle.endpoint, signal_body).await;
    let delete_body = soap::delete_shell(&handle.endpoint, &handle.shell_id);
    let _ = authenticated_post(&handle.client, &handle.endpoint, delete_body).await;
}

fn enrich_host_data(result: &CollectionResult, host_data: &mut HostData) {
    // Confident by construction: this only runs after a successful WinRM/NTLM
    // session against the target, which only Windows hosts speak.
    host_data.with_os_group(crate::server::hosts::r#impl::os::HostOsGroup::Windows);

    if let Some(name) = result.computer_name.as_ref().filter(|s| !s.is_empty()) {
        host_data.with_hostname_fallback(name.clone());
        host_data.with_sys_name(name.clone());
    }
    if let Some(manufacturer) = result.manufacturer.as_ref().filter(|s| !s.is_empty()) {
        host_data.with_manufacturer(manufacturer.clone());
    }
    if let Some(model) = result.model.as_ref().filter(|s| !s.is_empty()) {
        host_data.with_model(model.clone());
    }
    if let Some(serial) = result.serial_number.as_ref().filter(|s| !s.is_empty()) {
        host_data.with_serial_number(serial.clone());
    }

    let mut lines = Vec::new();
    if let Some(caption) = &result.os_caption {
        let mut line = caption.clone();
        if let Some(version) = &result.os_version {
            line.push_str(&format!(" (version {version}"));
            if let Some(build) = &result.os_build_number {
                line.push_str(&format!(", build {build}"));
            }
            line.push(')');
        }
        lines.push(line);
    }
    if let Some(arch) = &result.os_architecture {
        lines.push(format!("Architecture: {arch}"));
    }
    match (result.part_of_domain, &result.domain) {
        (Some(true), Some(domain)) if !domain.is_empty() => {
            lines.push(format!("Domain: {domain}"));
        }
        (Some(false), _) => lines.push("Workgroup (not domain-joined)".to_string()),
        _ => {}
    }
    if !lines.is_empty() {
        let description = lines.join("\n");
        host_data.with_sys_descr(truncate_utf8(description, MAX_STORED_DESCRIPTION));
    }
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

fn target_for(port: u16, use_tls: bool) -> WinRmTarget {
    WinRmTarget { port, use_tls }
}

fn port_type(port: u16, use_tls: bool) -> PortType {
    match (port, use_tls) {
        (5985, false) => PortType::WinRm,
        (5986, true) => PortType::WinRmHttps,
        _ => PortType::new_tcp(port),
    }
}

macro_rules! winrm_integration {
    ($integration:ident, $payload_variant:ident, $credential_ty:ty, $domain_expr:expr) => {
        pub struct $integration;

        #[async_trait]
        impl DiscoveryIntegration for $integration {
            /// OS, hardware and domain-membership details only; it builds no Interface rows.
            fn interface_view_scope(&self) -> InterfaceViewScope {
                InterfaceViewScope::NoInterfaces
            }

            fn credential_type(&self) -> CredentialQueryPayloadDiscriminants {
                CredentialQueryPayloadDiscriminants::$payload_variant
            }

            fn estimated_seconds(&self) -> u32 {
                20
            }

            fn timeout(&self) -> Duration {
                Duration::from_secs(90)
            }

            fn probe_gate_ports(&self, credential: &CredentialQueryPayload) -> Vec<PortType> {
                match credential {
                    CredentialQueryPayload::$payload_variant(c) => {
                        vec![port_type(c.port, c.use_tls)]
                    }
                    _ => vec![],
                }
            }

            async fn probe(&self, ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure> {
                let credential: &$credential_ty = match ctx.credential {
                    CredentialQueryPayload::$payload_variant(c) => c,
                    _ => return Err(ProbeFailure::malformed("expected a different credential type")),
                };
                if ctx.cancel.is_cancelled() {
                    return Err(ProbeFailure::cancelled());
                }
                let password = match &credential.password {
                    ResolvableSecret::Value { value } => value.clone(),
                    ResolvableSecret::FilePath { .. } => {
                        return Err(ProbeFailure::malformed("WinRM secret was not resolved"));
                    }
                };
                let target = target_for(credential.port, credential.use_tls);
                let domain: &str = ($domain_expr)(credential);
                let handle = tokio::select! {
                    _ = ctx.cancel.cancelled() => return Err(ProbeFailure::cancelled()),
                    result = open_shell(
                        ctx.ip,
                        &target,
                        credential.accept_invalid_certs,
                        domain,
                        &credential.username,
                        &password,
                    ) => result,
                };
                match handle {
                    Ok(handle) => Ok(ProbeSuccess {
                        client_probe: ClientProbe::WinRm,
                        ports: vec![port_type(credential.port, credential.use_tls)],
                        handle: Some(Box::new(handle)),
                    }),
                    Err(error) => {
                        tracing::debug!(ip = %ctx.ip, error = %error, "WinRM probe failed");
                        Err(ProbeFailure::rejected(
                            "WinRM connection, host verification, or authentication failed",
                        ))
                    }
                }
            }

            async fn execute(
                &self,
                ctx: &IntegrationContext<'_>,
                host_data: &mut HostData,
                _checkpoint: &Checkpoint<'_>,
            ) -> Result<Completeness, IntegrationFailure> {
                let handle = ctx
                    .probe_handle
                    .and_then(|value| value.downcast_ref::<WinRmProbeHandle>())
                    .ok_or_else(|| anyhow!("WinRM execute called without a probe handle"))?;
                let raw = run_collection_script(handle).await?;
                let result: CollectionResult = serde_json::from_str(raw.trim())
                    .map_err(|e| anyhow!("WinRM collection output was not valid JSON: {e}"))?;
                enrich_host_data(&result, host_data);
                Ok(Completeness::Complete)
            }
        }
    };
}

// Plain `fn` items rather than closures: function items are implicitly
// higher-ranked over their input lifetime (`for<'a> fn(&'a T) -> &'a str`),
// which is what the macro's generic call site needs. An inferred closure
// type here pins concrete (non-higher-ranked) lifetimes and fails to borrow-check.
fn no_domain(_c: &WindowsLocalAccountQueryCredential) -> &str {
    ""
}

fn domain_of(c: &WindowsDomainAccountQueryCredential) -> &str {
    c.domain.as_str()
}

winrm_integration!(
    WindowsLocalAccountIntegration,
    WindowsLocalAccount,
    WindowsLocalAccountQueryCredential,
    no_domain
);
winrm_integration!(
    WindowsDomainAccountIntegration,
    WindowsDomainAccount,
    WindowsDomainAccountQueryCredential,
    domain_of
);

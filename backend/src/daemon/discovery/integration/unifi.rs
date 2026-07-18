//! Bounded, read-only UniFi controller discovery.
//!
//! The configured HTTPS origin supplies the TLS identity, but reqwest resolves it
//! only to the host IP selected by Scanopy. Redirects are disabled and the only
//! mutating request permitted is the exact modern or legacy login endpoint.

use std::{collections::HashSet, net::SocketAddr, str::FromStr, sync::Arc, time::Duration};

use anyhow::{Error, anyhow};
use async_trait::async_trait;
use futures::StreamExt;
use mac_address::MacAddress;
use reqwest::{Method, StatusCode, header::HeaderMap};
use serde_json::{Value, json};

use crate::{
    daemon::discovery::service::ops::HostData,
    server::{
        credentials::r#impl::mapping::{
            CredentialQueryPayload, CredentialQueryPayloadDiscriminants, ResolvableSecret,
            UnifiApiType, UnifiQueryCredential, UnifiTlsPolicy,
        },
        hosts::r#impl::base::{Host, HostBase},
        interfaces::r#impl::base::{
            IfAdminStatus, IfOperStatus, Interface, InterfaceBase, if_type,
        },
        ip_addresses::r#impl::base::{IPAddress, IPAddressBase},
        ports::r#impl::base::PortType,
        services::r#impl::patterns::ClientProbe,
        shared::types::entities::EntitySource,
        subnets::r#impl::base::Subnet,
    },
};

use super::{DiscoveryIntegration, IntegrationContext, ProbeContext, ProbeFailure, ProbeSuccess};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const INTEGRATION_TIMEOUT: Duration = Duration::from_secs(90);
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const MAX_DEVICES: usize = 512;
const MAX_PORTS_PER_DEVICE: usize = 256;
const MAX_TEXT: usize = 256;

pub struct UnifiIntegration;

struct UnifiProbeHandle {
    client: UnifiClient,
}

#[async_trait]
impl DiscoveryIntegration for UnifiIntegration {
    fn credential_type(&self) -> CredentialQueryPayloadDiscriminants {
        CredentialQueryPayloadDiscriminants::Unifi
    }

    fn estimated_seconds(&self) -> u32 {
        20
    }

    fn timeout(&self) -> Duration {
        INTEGRATION_TIMEOUT
    }

    fn probe_gate_ports(&self, credential: &CredentialQueryPayload) -> Vec<PortType> {
        match credential {
            CredentialQueryPayload::Unifi(value) => {
                value.port().map(port_type).into_iter().collect()
            }
            _ => Vec::new(),
        }
    }

    async fn probe(&self, ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure> {
        let raw = match ctx.credential {
            CredentialQueryPayload::Unifi(value) => value,
            _ => return Err(failure("expected UniFi credential")),
        };
        if ctx.cancel.is_cancelled() {
            return Err(failure("cancelled"));
        }
        let credential = resolve_unifi_credential(ctx.credential, ctx.ip)?;
        debug_assert_eq!(raw.server_name, credential.server_name);

        let transport = ReqwestTransport::new(ctx.ip, &credential).map_err(|error| {
            tracing::debug!(ip = %ctx.ip, error = %error, "UniFi endpoint setup failed");
            failure("UniFi endpoint configuration is invalid")
        })?;
        let mut client = UnifiClient::new(credential, Arc::new(transport));
        let login = tokio::select! {
            _ = ctx.cancel.cancelled() => return Err(failure("cancelled")),
            result = tokio::time::timeout(REQUEST_TIMEOUT, client.login()) => result,
        };
        match login {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                tracing::debug!(ip = %ctx.ip, error = %error, "UniFi authentication probe failed");
                return Err(failure("UniFi authentication failed"));
            }
            Err(_) => return Err(failure("UniFi authentication timed out")),
        }

        Ok(ProbeSuccess {
            client_probe: ClientProbe::Unifi,
            ports: client
                .credential
                .port()
                .map(port_type)
                .into_iter()
                .collect(),
            handle: Some(Box::new(UnifiProbeHandle { client })),
        })
    }

    async fn execute(
        &self,
        ctx: &IntegrationContext<'_>,
        host_data: &mut HostData,
    ) -> Result<(), Error> {
        let handle = ctx
            .probe_handle
            .and_then(|value| value.downcast_ref::<UnifiProbeHandle>())
            .ok_or_else(|| anyhow!("UniFi execute called without a probe handle"))?;
        let response = tokio::select! {
            _ = ctx.cancel.cancelled() => return Err(anyhow!("discovery was cancelled")),
            result = tokio::time::timeout(REQUEST_TIMEOUT, handle.client.devices()) => result,
        }
        .map_err(|_| anyhow!("UniFi device collection timed out"))??;

        let devices = normalize_devices(&response)?;
        apply_devices(ctx, host_data, &handle.client.credential, &devices).await
    }
}

fn resolve_unifi_credential(
    payload: &CredentialQueryPayload,
    ip: std::net::IpAddr,
) -> Result<UnifiQueryCredential, ProbeFailure> {
    let resolved = payload.resolve_file_paths().map_err(|_| {
        // File resolution errors can contain an operator's exact local path.
        // The category is actionable without copying that path into daemon logs.
        tracing::debug!(%ip, "UniFi credential material could not be resolved");
        failure("UniFi credential material could not be resolved")
    })?;
    match resolved {
        CredentialQueryPayload::Unifi(credential) => Ok(credential),
        _ => Err(failure("expected UniFi credential")),
    }
}

fn failure(message: impl Into<String>) -> ProbeFailure {
    ProbeFailure {
        message: message.into(),
    }
}

fn port_type(port: u16) -> PortType {
    match port {
        443 => PortType::Https,
        8443 => PortType::Https8443,
        value => PortType::new_tcp(value),
    }
}

#[derive(Debug)]
struct TransportResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
}

#[async_trait]
trait UnifiTransport: Send + Sync {
    async fn request(
        &self,
        method: Method,
        path_and_query: &str,
        headers: HeaderMap,
        body: Option<Vec<u8>>,
    ) -> Result<TransportResponse, Error>;
}

struct ReqwestTransport {
    origin: url::Url,
    client: reqwest::Client,
}

impl ReqwestTransport {
    fn new(ip: std::net::IpAddr, credential: &UnifiQueryCredential) -> Result<Self, Error> {
        let origin = validated_origin(credential)?;
        let port = origin
            .port_or_known_default()
            .ok_or_else(|| anyhow!("UniFi URL has no port"))?;
        let client = reqwest::Client::builder()
            .https_only(true)
            // A proxy would resolve and connect on our behalf, bypassing the
            // daemon-assigned IP boundary established by `resolve` below.
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(REQUEST_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .danger_accept_invalid_certs(
                credential.tls_policy == UnifiTlsPolicy::AllowInvalidCertificate,
            )
            .resolve(&credential.server_name, SocketAddr::new(ip, port))
            .build()?;
        Ok(Self { origin, client })
    }

    #[cfg(test)]
    fn new_with_root(
        ip: std::net::IpAddr,
        credential: &UnifiQueryCredential,
        root_pem: &[u8],
    ) -> Result<Self, Error> {
        let origin = validated_origin(credential)?;
        let port = origin
            .port_or_known_default()
            .ok_or_else(|| anyhow!("UniFi URL has no port"))?;
        let root = reqwest::Certificate::from_pem(root_pem)?;
        let client = reqwest::Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(REQUEST_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .add_root_certificate(root)
            .resolve(&credential.server_name, SocketAddr::new(ip, port))
            .build()?;
        Ok(Self { origin, client })
    }
}

#[async_trait]
impl UnifiTransport for ReqwestTransport {
    async fn request(
        &self,
        method: Method,
        path_and_query: &str,
        headers: HeaderMap,
        body: Option<Vec<u8>>,
    ) -> Result<TransportResponse, Error> {
        validate_request(&method, path_and_query)?;
        let url = self.origin.join(path_and_query)?;
        if url.origin() != self.origin.origin() {
            return Err(anyhow!("UniFi request changed origin"));
        }
        let mut request = self.client.request(method, url).headers(headers);
        if let Some(body) = body {
            request = request
                .header("content-type", "application/json")
                .body(body);
        }
        let response = request.send().await?;
        let status = response.status();
        let headers = response.headers().clone();
        let mut stream = response.bytes_stream();
        let mut body = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
                return Err(anyhow!("UniFi response exceeds the size limit"));
            }
            body.extend_from_slice(&chunk);
        }
        Ok(TransportResponse {
            status,
            headers,
            body,
        })
    }
}

fn validated_origin(credential: &UnifiQueryCredential) -> Result<url::Url, Error> {
    if credential.controller_url.len() > 2_048
        || !valid_dns_name(&credential.server_name)
        || credential.username.trim().is_empty()
        || credential.username.len() > 256
        || credential.username.chars().any(char::is_control)
        || resolved_secret(&credential.password)
            .is_ok_and(|value| value.len() < 4 || value.len() > 64 * 1024)
    {
        return Err(anyhow!("invalid UniFi endpoint"));
    }
    let url = url::Url::parse(&credential.controller_url)?;
    if url.scheme() != "https"
        || url.host_str() != Some(credential.server_name.as_str())
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !matches!(url.path(), "" | "/")
        || !valid_site(&credential.site)
    {
        return Err(anyhow!("invalid UniFi endpoint"));
    }
    Ok(url)
}

fn valid_dns_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && !value.ends_with('.')
        && value.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

fn valid_site(site: &str) -> bool {
    !site.is_empty()
        && site.len() <= 128
        && site
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn validate_request(method: &Method, path: &str) -> Result<(), Error> {
    let parsed = url::Url::parse(&format!("https://fixed.invalid{path}"))?;
    if !path.starts_with('/')
        || path.starts_with("//")
        || parsed.host_str() != Some("fixed.invalid")
        || parsed.fragment().is_some()
    {
        return Err(anyhow!("UniFi client refused an invalid request target"));
    }
    let login = method == Method::POST
        && matches!(parsed.path(), "/api/auth/login" | "/api/login")
        && parsed.query().is_none();
    let segments = parsed.path().split('/').collect::<Vec<_>>();
    let read_path = matches!(
        segments.as_slice(),
        ["", "api", "s", _, "stat", "device"]
            | ["", "proxy", "network", "api", "s", _, "stat", "device"]
    );
    let query = parsed.query_pairs().collect::<Vec<_>>();
    let expected_limit = MAX_DEVICES.to_string();
    let read_query = query.len() == 2
        && query
            .iter()
            .any(|(key, value)| key == "_start" && value == "0")
        && query
            .iter()
            .any(|(key, value)| key == "_limit" && value == expected_limit.as_str());
    let read = method == Method::GET && read_path && read_query;
    if !login && !read {
        return Err(anyhow!("UniFi client refused a non-read-only operation"));
    }
    Ok(())
}

struct UnifiClient {
    credential: UnifiQueryCredential,
    transport: Arc<dyn UnifiTransport>,
    session_headers: HeaderMap,
}

impl UnifiClient {
    fn new(credential: UnifiQueryCredential, transport: Arc<dyn UnifiTransport>) -> Self {
        Self {
            credential,
            transport,
            session_headers: HeaderMap::new(),
        }
    }

    async fn login(&mut self) -> Result<(), Error> {
        let password = resolved_secret(&self.credential.password)?;
        let path = match self.credential.api_type {
            UnifiApiType::Modern => "/api/auth/login",
            UnifiApiType::Legacy => "/api/login",
        };
        let body = match self.credential.api_type {
            UnifiApiType::Modern => json!({
                "username": self.credential.username,
                "password": password,
                "rememberMe": false
            }),
            UnifiApiType::Legacy => json!({
                "username": self.credential.username,
                "password": password
            }),
        };
        let response = self
            .transport
            .request(
                Method::POST,
                path,
                HeaderMap::new(),
                Some(serde_json::to_vec(&body)?),
            )
            .await?;
        if matches!(
            response.status,
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
        ) {
            return Err(anyhow!("UniFi credential was rejected"));
        }
        if matches!(response.status.as_u16(), 409 | 412) || requires_mfa(&response.body) {
            return Err(anyhow!("UniFi controller requires unsupported MFA"));
        }
        if !response.status.is_success() || legacy_error(&response.body) {
            return Err(anyhow!("UniFi login failed"));
        }
        if let Some(cookie) = session_cookie(&response.headers) {
            self.session_headers.insert("cookie", cookie.parse()?);
        }
        if let Some(csrf) = response.headers.get("x-csrf-token") {
            self.session_headers.insert("x-csrf-token", csrf.clone());
        }
        Ok(())
    }

    async fn devices(&self) -> Result<Vec<Value>, Error> {
        let prefix = match self.credential.api_type {
            UnifiApiType::Modern => "/proxy/network",
            UnifiApiType::Legacy => "",
        };
        let path = format!(
            "{prefix}/api/s/{}/stat/device?_start=0&_limit={MAX_DEVICES}",
            self.credential.site
        );
        let response = self
            .transport
            .request(Method::GET, &path, self.session_headers.clone(), None)
            .await?;
        if matches!(
            response.status,
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
        ) {
            return Err(anyhow!("UniFi controller denied read access"));
        }
        if !response.status.is_success() {
            return Err(anyhow!("UniFi device read failed"));
        }
        let document: Value = serde_json::from_slice(&response.body)
            .map_err(|_| anyhow!("UniFi controller returned invalid JSON"))?;
        let data = document
            .get("data")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("UniFi response does not contain a data list"))?;
        if data.len() > MAX_DEVICES {
            return Err(anyhow!("UniFi device result exceeds the item limit"));
        }
        Ok(data.clone())
    }
}

fn resolved_secret(secret: &ResolvableSecret) -> Result<&str, Error> {
    match secret {
        ResolvableSecret::Value { value } => Ok(value),
        ResolvableSecret::FilePath { .. } => Err(anyhow!("UniFi secret file was not resolved")),
    }
}

fn session_cookie(headers: &HeaderMap) -> Option<String> {
    let values = headers
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .filter_map(|value| value.split(';').next())
        .filter(|value| value.len() <= 4096 && !value.chars().any(char::is_control))
        .collect::<Vec<_>>();
    (!values.is_empty()).then(|| values.join("; "))
}

fn requires_mfa(body: &[u8]) -> bool {
    let Ok(value) = serde_json::from_slice::<Value>(body) else {
        return false;
    };
    ["error", "message", "code"].iter().any(|key| {
        value.get(key).and_then(Value::as_str).is_some_and(|text| {
            let text = text.to_ascii_lowercase();
            text.contains("mfa") || text.contains("2fa")
        })
    })
}

fn legacy_error(body: &[u8]) -> bool {
    serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|value| value.get("meta").cloned())
        .and_then(|value| value.get("rc").cloned())
        .and_then(|value| value.as_str().map(str::to_string))
        .is_some_and(|value| value == "error")
}

#[derive(Debug, Clone)]
struct UnifiDevice {
    name: String,
    ip: Option<std::net::IpAddr>,
    mac: Option<MacAddress>,
    model: Option<String>,
    version: Option<String>,
    serial: Option<String>,
    ports: Vec<UnifiPort>,
}

#[derive(Debug, Clone)]
struct UnifiPort {
    index: i32,
    name: String,
    up: bool,
    speed_bps: Option<i64>,
}

fn normalize_devices(values: &[Value]) -> Result<Vec<UnifiDevice>, Error> {
    if values.len() > MAX_DEVICES {
        return Err(anyhow!("UniFi device result exceeds the item limit"));
    }
    let mut devices = Vec::new();
    for value in values {
        let Some(object) = value.as_object() else {
            continue;
        };
        let Some(_) = bounded(object.get("_id").and_then(Value::as_str), MAX_TEXT) else {
            continue;
        };
        let name = bounded(object.get("name").and_then(Value::as_str), 100)
            .unwrap_or_else(|| "UniFi Device".to_string());
        let ip = object
            .get("ip")
            .and_then(Value::as_str)
            .and_then(|value| value.parse().ok());
        let mac = object
            .get("mac")
            .and_then(Value::as_str)
            .and_then(|value| MacAddress::from_str(value).ok());
        let port_values = object
            .get("port_table")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let mut ports = Vec::new();
        for port in port_values.iter().take(MAX_PORTS_PER_DEVICE) {
            let Some(port) = port.as_object() else {
                continue;
            };
            let Some(index) = port
                .get("port_idx")
                .and_then(Value::as_i64)
                .and_then(|value| i32::try_from(value).ok())
                .filter(|value| *value > 0)
            else {
                continue;
            };
            let name = bounded(port.get("name").and_then(Value::as_str), 128)
                .unwrap_or_else(|| format!("Port {index}"));
            let speed_bps = port
                .get("speed")
                .and_then(Value::as_i64)
                .filter(|value| (0..=1_000_000).contains(value))
                .and_then(|value| value.checked_mul(1_000_000));
            ports.push(UnifiPort {
                index,
                name,
                up: port.get("up").and_then(Value::as_bool).unwrap_or(false),
                speed_bps,
            });
        }
        devices.push(UnifiDevice {
            name,
            ip,
            mac,
            model: bounded(object.get("model").and_then(Value::as_str), MAX_TEXT),
            version: bounded(object.get("version").and_then(Value::as_str), MAX_TEXT),
            serial: bounded(object.get("serial").and_then(Value::as_str), MAX_TEXT),
            ports,
        });
    }
    Ok(devices)
}

fn bounded(value: Option<&str>, max: usize) -> Option<String> {
    let value = value?.trim();
    (!value.is_empty() && value.len() <= max && !value.chars().any(char::is_control))
        .then(|| value.to_string())
}

async fn apply_devices(
    ctx: &IntegrationContext<'_>,
    host_data: &mut HostData,
    credential: &UnifiQueryCredential,
    devices: &[UnifiDevice],
) -> Result<(), Error> {
    let network_id = ctx.ops.network_id().await?;
    host_data.with_management_url(credential.controller_url.clone());

    for device in devices {
        if ctx.cancel.is_cancelled() {
            return Err(anyhow!("discovery was cancelled"));
        }
        let Some(ip) = device.ip else {
            continue;
        };
        let interfaces = device_interfaces(device, network_id);
        if ip == ctx.ip {
            enrich_host(host_data, device, interfaces);
            continue;
        }
        let Some(subnet) = matching_subnet(ip, ctx.created_subnets, ctx.scanning_subnet) else {
            tracing::debug!(ip = %ip, "Skipping UniFi device outside approved discovery subnets");
            continue;
        };
        let host = Host::new(HostBase {
            name: device.name.clone(),
            network_id,
            source: EntitySource::Discovery,
            management_url: Some(credential.controller_url.clone()),
            manufacturer: Some("Ubiquiti".to_string()),
            model: device.model.clone(),
            serial_number: device.serial.clone(),
            sys_descr: device
                .version
                .as_ref()
                .map(|version| format!("UniFi {version}")),
            chassis_id: device.mac.map(|mac| mac.to_string()),
            ..Default::default()
        });
        let address = IPAddress::new(IPAddressBase {
            network_id,
            host_id: uuid::Uuid::nil(),
            subnet_id: subnet.id,
            ip_address: ip,
            mac_address: device.mac,
            name: None,
            position: 0,
        });
        ctx.ops
            .create_host(
                host,
                vec![address],
                Vec::new(),
                Vec::new(),
                interfaces,
                Vec::new(),
                false,
                ctx.cancel,
            )
            .await?;
    }
    Ok(())
}

fn enrich_host(host_data: &mut HostData, device: &UnifiDevice, interfaces: Vec<Interface>) {
    host_data.with_manufacturer("Ubiquiti".to_string());
    if let Some(value) = &device.model {
        host_data.with_model(value.clone());
    }
    if let Some(value) = &device.serial {
        host_data.with_serial_number(value.clone());
    }
    if let Some(value) = device.mac {
        host_data.with_chassis_id(value.to_string());
        if let Some(ip) = device.ip {
            host_data.with_mac_for_ip(ip, value);
        }
    }
    if let Some(value) = &device.version {
        host_data.with_sys_descr(format!("UniFi {value}"));
    }
    merge_controller_interfaces(host_data, interfaces);
}

fn device_interfaces(device: &UnifiDevice, network_id: uuid::Uuid) -> Vec<Interface> {
    device
        .ports
        .iter()
        .map(|port| {
            Interface::new(InterfaceBase {
                host_id: uuid::Uuid::nil(),
                network_id,
                if_index: port.index,
                if_descr: port.name.clone(),
                if_name: Some(port.name.clone()),
                if_type: if_type::ETHERNET_CSMA_CD,
                speed_bps: port.speed_bps,
                admin_status: IfAdminStatus::Up,
                oper_status: if port.up {
                    IfOperStatus::Up
                } else {
                    IfOperStatus::Down
                },
                // The device chassis MAC is not a per-port MAC. Repeating it on
                // every interface would create false identity correlations.
                mac_address: None,
                ..Default::default()
            })
        })
        .collect()
}

/// UniFi's port table is useful enrichment, but it is neither a complete
/// ifTable nor an authoritative L2-neighbor source. If SNMP already supplied
/// interfaces, enrich only exact ifIndex/name matches so the server never sees
/// duplicate live interfaces and preserve SNMP's completeness decision. When
/// there is no authoritative interface set, retain the bounded controller ports
/// but mark the set partial so it can never prune previously known interfaces.
fn merge_controller_interfaces(host_data: &mut HostData, interfaces: Vec<Interface>) {
    if host_data.interfaces.is_empty() {
        host_data.interfaces = interfaces;
        host_data.interfaces_complete = false;
        return;
    }

    let mut claimed = HashSet::new();
    for controller in interfaces {
        let controller_name = controller.base.if_name.as_deref();
        let existing_index = host_data
            .interfaces
            .iter()
            .enumerate()
            .find(|(position, existing)| {
                if claimed.contains(position) || existing.base.if_index != controller.base.if_index
                {
                    return false;
                }
                match (existing.base.if_name.as_deref(), controller_name) {
                    (Some(existing_name), Some(name)) => existing_name.eq_ignore_ascii_case(name),
                    // ifIndex is the only shared identifier when either source
                    // omitted ifName; never fall back across conflicting names.
                    _ => true,
                }
            })
            .map(|(position, _)| position);
        if let Some(position) = existing_index {
            claimed.insert(position);
            let existing = &mut host_data.interfaces[position];
            if existing.base.if_name.is_none() {
                existing.base.if_name = controller.base.if_name;
            }
            if existing.base.speed_bps.is_none() {
                existing.base.speed_bps = controller.base.speed_bps;
            }
        }
    }
}

fn matching_subnet<'a>(
    ip: std::net::IpAddr,
    created: &'a [Subnet],
    scanning: Option<&'a Subnet>,
) -> Option<&'a Subnet> {
    created
        .iter()
        .find(|subnet| subnet.base.cidr.contains(&ip))
        .or_else(|| scanning.filter(|subnet| subnet.base.cidr.contains(&ip)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        daemon::{
            discovery::{
                buffer::EntityBuffer,
                service::{base::DaemonDiscoveryService, ops::DiscoveryOps},
            },
            shared::config::{AppConfig, ConfigStore},
            utils::base::create_system_utils,
        },
        server::{
            daemons::r#impl::base::DaemonMode,
            discovery::r#impl::types::{DiscoveryType, HostNamingFallback},
            hosts::r#impl::api::HostResponse,
        },
    };
    use std::{
        collections::VecDeque,
        io::{Cursor, Read, Write},
        net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener},
        path::PathBuf,
        sync::{Mutex, mpsc},
        thread,
    };
    use tokio_util::sync::CancellationToken;

    struct LoopbackTlsServer {
        address: SocketAddr,
        root_pem: Vec<u8>,
        sni: mpsc::Receiver<Option<String>>,
        expected_connections: usize,
        worker: thread::JoinHandle<()>,
    }

    impl LoopbackTlsServer {
        fn spawn(response: Vec<u8>) -> Self {
            Self::spawn_sequence(vec![response])
        }

        fn spawn_sequence(responses: Vec<Vec<u8>>) -> Self {
            let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
            let address = listener.local_addr().unwrap();
            let expected_connections = responses.len();
            let (sni_sender, sni) = mpsc::channel();
            let rcgen::CertifiedKey { cert, key_pair } =
                rcgen::generate_simple_self_signed(vec!["controller.example.test".to_string()])
                    .unwrap();
            let certificate_pem = cert.pem();
            let private_key_pem = key_pair.serialize_pem();
            let root_pem = certificate_pem.as_bytes().to_vec();
            let worker = thread::spawn(move || {
                let certificates = rustls_pemfile::certs(&mut Cursor::new(certificate_pem))
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap();
                let private_key = rustls_pemfile::private_key(&mut Cursor::new(private_key_pem))
                    .unwrap()
                    .unwrap();
                let config = rustls::ServerConfig::builder()
                    .with_no_client_auth()
                    .with_single_cert(certificates, private_key)
                    .unwrap();
                let config = Arc::new(config);
                for response in responses {
                    let (socket, _) = listener.accept().unwrap();
                    socket
                        .set_read_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    socket
                        .set_write_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    let connection = rustls::ServerConnection::new(config.clone()).unwrap();
                    let mut stream = rustls::StreamOwned::new(connection, socket);
                    let mut request = Vec::new();
                    let mut buffer = [0_u8; 4096];
                    loop {
                        match stream.read(&mut buffer) {
                            Ok(0) => break,
                            Ok(count) => {
                                request.extend_from_slice(&buffer[..count]);
                                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                                    break;
                                }
                            }
                            // A verification-failure test intentionally makes the
                            // client abort after receiving the server certificate.
                            Err(_) => break,
                        }
                    }
                    let _ = sni_sender.send(stream.conn.server_name().map(ToString::to_string));
                    if stream.write_all(&response).is_ok() {
                        let _ = stream.flush();
                    }
                    if request.is_empty() {
                        return;
                    }
                }
            });
            Self {
                address,
                root_pem,
                sni,
                expected_connections,
                worker,
            }
        }

        fn finish(self) -> Option<String> {
            let sni = self.sni.recv_timeout(Duration::from_secs(5)).unwrap();
            self.worker.join().unwrap();
            sni
        }

        fn finish_all(self) -> Vec<Option<String>> {
            let sni = (0..self.expected_connections)
                .map(|_| self.sni.recv_timeout(Duration::from_secs(5)).unwrap())
                .collect();
            self.worker.join().unwrap();
            sni
        }
    }

    fn loopback_credential(port: u16) -> UnifiQueryCredential {
        let mut value = credential("fixture-password");
        value.controller_url = format!("https://controller.example.test:{port}");
        value.server_name = "controller.example.test".to_string();
        value
    }

    fn http_response(status: &str, headers: &[(&str, &str)], body: &[u8]) -> Vec<u8> {
        let mut response = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n",
            body.len()
        )
        .into_bytes();
        for (name, value) in headers {
            response.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
        }
        response.extend_from_slice(b"\r\n");
        response.extend_from_slice(body);
        response
    }

    struct MockTransport {
        responses: Mutex<VecDeque<TransportResponse>>,
        requests: Mutex<Vec<(Method, String, HeaderMap, Option<Vec<u8>>)>>,
    }

    #[async_trait]
    impl UnifiTransport for MockTransport {
        async fn request(
            &self,
            method: Method,
            path: &str,
            headers: HeaderMap,
            body: Option<Vec<u8>>,
        ) -> Result<TransportResponse, Error> {
            self.requests
                .lock()
                .unwrap()
                .push((method, path.to_string(), headers, body));
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| anyhow!("missing mock response"))
        }
    }

    fn response(status: u16, body: Value) -> TransportResponse {
        TransportResponse {
            status: StatusCode::from_u16(status).unwrap(),
            headers: HeaderMap::new(),
            body: serde_json::to_vec(&body).unwrap(),
        }
    }

    fn credential(password: &str) -> UnifiQueryCredential {
        UnifiQueryCredential {
            controller_url: "https://controller.example.com:8443".to_string(),
            server_name: "controller.example.com".to_string(),
            site: "default".to_string(),
            api_type: UnifiApiType::Modern,
            tls_policy: UnifiTlsPolicy::Verify,
            username: "reader".to_string(),
            password: ResolvableSecret::Value {
                value: password.to_string(),
            },
        }
    }

    #[tokio::test]
    async fn modern_login_and_device_read_use_exact_allowlisted_paths() {
        let mut login = response(200, json!({}));
        login.headers.insert(
            "set-cookie",
            "TOKEN=fixture; Secure; HttpOnly".parse().unwrap(),
        );
        login
            .headers
            .insert("x-csrf-token", "csrf-fixture".parse().unwrap());
        let transport = Arc::new(MockTransport {
            responses: Mutex::new(VecDeque::from([
                login,
                response(200, json!({"data": [{"_id": "switch"}]})),
            ])),
            requests: Mutex::new(Vec::new()),
        });
        let mut client = UnifiClient::new(credential("fixture-password"), transport.clone());
        client.login().await.unwrap();
        assert_eq!(client.devices().await.unwrap().len(), 1);
        let requests = transport.requests.lock().unwrap();
        assert_eq!(requests[0].0, Method::POST);
        assert_eq!(requests[0].1, "/api/auth/login");
        assert_eq!(requests[1].0, Method::GET);
        assert_eq!(
            requests[1].1,
            "/proxy/network/api/s/default/stat/device?_start=0&_limit=512"
        );
        assert_eq!(requests[1].2.get("cookie").unwrap(), "TOKEN=fixture");
        assert!(
            String::from_utf8_lossy(requests[0].3.as_ref().unwrap()).contains("fixture-password")
        );
    }

    #[tokio::test]
    async fn loopback_https_verifies_test_ca_and_uses_fixed_ip_with_dns_sni() {
        let server = LoopbackTlsServer::spawn(http_response("200 OK", &[], b"{}"));
        let credential = loopback_credential(server.address.port());
        let transport = ReqwestTransport::new_with_root(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            &credential,
            &server.root_pem,
        )
        .unwrap();

        let response = transport
            .request(
                Method::POST,
                "/api/auth/login",
                HeaderMap::new(),
                Some(b"{}".to_vec()),
            )
            .await
            .unwrap();

        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(response.body, b"{}");
        assert_eq!(server.finish().as_deref(), Some("controller.example.test"));
    }

    #[tokio::test]
    async fn loopback_https_rejects_untrusted_certificate() {
        let server = LoopbackTlsServer::spawn(http_response("200 OK", &[], b"{}"));
        let credential = loopback_credential(server.address.port());
        let transport =
            ReqwestTransport::new(IpAddr::V4(Ipv4Addr::LOCALHOST), &credential).unwrap();

        let result = transport
            .request(
                Method::POST,
                "/api/auth/login",
                HeaderMap::new(),
                Some(b"{}".to_vec()),
            )
            .await;

        assert!(
            result.is_err(),
            "an untrusted controller certificate was accepted"
        );
        assert_eq!(server.finish().as_deref(), Some("controller.example.test"));
    }

    #[tokio::test]
    async fn loopback_https_returns_redirect_without_following_it() {
        let server = LoopbackTlsServer::spawn(http_response(
            "302 Found",
            &[("Location", "https://attacker.invalid/api/auth/login")],
            b"redirect denied",
        ));
        let credential = loopback_credential(server.address.port());
        let transport = ReqwestTransport::new_with_root(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            &credential,
            &server.root_pem,
        )
        .unwrap();

        let response = transport
            .request(
                Method::POST,
                "/api/auth/login",
                HeaderMap::new(),
                Some(b"{}".to_vec()),
            )
            .await
            .unwrap();

        assert_eq!(response.status, StatusCode::FOUND);
        assert_eq!(server.finish().as_deref(), Some("controller.example.test"));
    }

    #[tokio::test]
    async fn loopback_https_rejects_streaming_body_above_limit() {
        let body = vec![b'x'; MAX_RESPONSE_BYTES + 1];
        let server = LoopbackTlsServer::spawn(http_response("200 OK", &[], &body));
        let credential = loopback_credential(server.address.port());
        let transport = ReqwestTransport::new_with_root(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            &credential,
            &server.root_pem,
        )
        .unwrap();

        let error = transport
            .request(
                Method::GET,
                "/proxy/network/api/s/default/stat/device?_start=0&_limit=512",
                HeaderMap::new(),
                None,
            )
            .await
            .unwrap_err()
            .to_string();

        assert!(error.contains("size limit"), "unexpected error: {error}");
        assert_eq!(server.finish().as_deref(), Some("controller.example.test"));
    }

    #[tokio::test]
    async fn probe_execute_enriches_snmp_host_and_buffers_normalized_controller_device()
    -> Result<(), Error> {
        let controller_ip = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let discovered_ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2));
        let device_document = json!({
            "data": [
                {
                    "_id": "controller-switch",
                    "name": "Controller Switch",
                    "ip": controller_ip.to_string(),
                    "mac": "02:00:00:00:00:01",
                    "model": "USW-Pro-24",
                    "version": "7.1.26",
                    "serial": "SWITCH-SERIAL",
                    "port_table": [{
                        "port_idx": 7,
                        "name": "ETH7",
                        "up": true,
                        "speed": 1000
                    }]
                },
                {
                    "_id": "controller-ap",
                    "name": "Fixture AP",
                    "ip": discovered_ip.to_string(),
                    "mac": "02:00:00:00:00:02",
                    "model": "U7-Pro",
                    "version": "8.0.1",
                    "serial": "AP-SERIAL",
                    "port_table": [{
                        "port_idx": 1,
                        "name": "eth0",
                        "up": true,
                        "speed": 2500
                    }]
                }
            ]
        });
        let server = LoopbackTlsServer::spawn_sequence(vec![
            http_response(
                "200 OK",
                &[("Set-Cookie", "TOKEN=fixture; Secure; HttpOnly")],
                b"{}",
            ),
            http_response(
                "200 OK",
                &[],
                &serde_json::to_vec(&device_document).unwrap(),
            ),
        ]);

        let mut credential = loopback_credential(server.address.port());
        credential.tls_policy = UnifiTlsPolicy::AllowInvalidCertificate;
        let payload = CredentialQueryPayload::Unifi(credential.clone());
        let cancel = CancellationToken::new();
        let utils = create_system_utils();
        let integration = UnifiIntegration;
        let probe = integration
            .probe(&ProbeContext {
                ip: controller_ip,
                credential: &payload,
                credential_id: None,
                cancel: &cancel,
                utils: &utils,
            })
            .await
            .map_err(|failure| anyhow!(failure.message))?;
        assert_eq!(probe.client_probe, ClientProbe::Unifi);
        let probe_handle = probe.handle.expect("probe should retain its session");

        let network_id = uuid::Uuid::new_v4();
        let subnet_id = uuid::Uuid::new_v4();
        let mut config = AppConfig::default();
        config.network_id = Some(network_id);
        config.mode = DaemonMode::ServerPoll;
        config.server_url = None;
        let config_store = Arc::new(ConfigStore::new(
            PathBuf::from("unused-unifi-integration-test.json"),
            config,
        ));
        let entity_buffer = Arc::new(EntityBuffer::new());
        let service = DaemonDiscoveryService::new(config_store, entity_buffer.clone());
        let ops = DiscoveryOps::new(&service, DiscoveryType::default());

        let mut scanning_subnet = Subnet::default();
        scanning_subnet.id = subnet_id;
        scanning_subnet.base.network_id = network_id;
        scanning_subnet.base.cidr = "127.0.0.0/8".parse().unwrap();
        let host = Host::new(HostBase {
            name: "SNMP Controller".to_string(),
            network_id,
            manufacturer: Some("SNMP authoritative vendor".to_string()),
            ..Default::default()
        });
        let host_id = host.id;
        let controller_address = IPAddress::new(IPAddressBase {
            network_id,
            host_id,
            subnet_id,
            ip_address: controller_ip,
            mac_address: None,
            name: None,
            position: 0,
        });
        let snmp_interface = Interface::new(InterfaceBase {
            host_id,
            network_id,
            if_index: 7,
            if_descr: "SNMP authoritative description".to_string(),
            if_name: Some("eth7".to_string()),
            speed_bps: None,
            ..Default::default()
        });
        let mut host_data = HostData::new(
            host,
            Vec::new(),
            Vec::new(),
            vec![controller_address],
            vec![snmp_interface],
            Vec::new(),
        );
        let context = IntegrationContext {
            ip: controller_ip,
            credential: &payload,
            credential_id: None,
            cancel: &cancel,
            ops: &ops,
            utils: &utils,
            probe_handle: Some(probe_handle.as_ref()),
            matched_services: &[],
            open_ports: &[],
            endpoint_responses: &[],
            host_id,
            host_naming_fallback: HostNamingFallback::default(),
            created_subnets: std::slice::from_ref(&scanning_subnet),
            accept_invalid_certs: false,
            scanning_subnet: Some(&scanning_subnet),
        };

        let confirm_buffered_host = async {
            tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    let mut pending = entity_buffer.get_pending().await.hosts;
                    if let Some(request) = pending.pop() {
                        let response = HostResponse::from_host_with_children(
                            request.host.clone(),
                            request.ip_addresses.clone(),
                            request.ports.clone(),
                            request.services.clone(),
                            request.interfaces.clone(),
                        );
                        entity_buffer
                            .mark_host_created(request.host.id, response)
                            .await;
                        return request;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .map_err(|_| anyhow!("UniFi execute did not reach the entity buffer"))
        };
        let ((), buffered) = tokio::try_join!(
            integration.execute(&context, &mut host_data),
            confirm_buffered_host
        )?;

        assert_eq!(host_data.host.base.name, "SNMP Controller");
        assert_eq!(
            host_data.host.base.manufacturer.as_deref(),
            Some("SNMP authoritative vendor")
        );
        assert_eq!(host_data.host.base.model.as_deref(), Some("USW-Pro-24"));
        assert_eq!(
            host_data.host.base.serial_number.as_deref(),
            Some("SWITCH-SERIAL")
        );
        assert_eq!(
            host_data.ip_addresses[0]
                .base
                .mac_address
                .unwrap()
                .to_string(),
            "02:00:00:00:00:01"
        );
        assert_eq!(host_data.interfaces.len(), 1);
        assert_eq!(
            host_data.interfaces[0].base.if_descr,
            "SNMP authoritative description"
        );
        assert_eq!(
            host_data.interfaces[0].base.if_name.as_deref(),
            Some("eth7")
        );
        assert_eq!(host_data.interfaces[0].base.speed_bps, Some(1_000_000_000));
        assert!(host_data.interfaces_complete);

        assert_eq!(buffered.host.base.name, "Fixture AP");
        assert_eq!(buffered.host.base.network_id, network_id);
        assert_eq!(buffered.host.base.manufacturer.as_deref(), Some("Ubiquiti"));
        assert_eq!(buffered.host.base.model.as_deref(), Some("U7-Pro"));
        assert_eq!(
            buffered.host.base.serial_number.as_deref(),
            Some("AP-SERIAL")
        );
        assert_eq!(buffered.ip_addresses.len(), 1);
        assert_eq!(buffered.ip_addresses[0].base.ip_address, discovered_ip);
        assert_eq!(buffered.ip_addresses[0].base.subnet_id, subnet_id);
        assert_eq!(buffered.interfaces.len(), 1);
        assert_eq!(buffered.interfaces[0].base.if_index, 1);
        assert_eq!(buffered.interfaces[0].base.if_name.as_deref(), Some("eth0"));
        assert_eq!(buffered.interfaces[0].base.speed_bps, Some(2_500_000_000));
        assert!(!buffered.interfaces_complete);
        assert_eq!(
            server.finish_all(),
            vec![
                Some("controller.example.test".to_string()),
                Some("controller.example.test".to_string())
            ]
        );
        Ok(())
    }

    #[test]
    fn request_boundary_refuses_writes_redirect_targets_and_wrong_login_paths() {
        assert!(validate_request(&Method::POST, "/api/auth/login").is_ok());
        assert!(
            validate_request(
                &Method::GET,
                "/api/s/default/stat/device?_start=0&_limit=512"
            )
            .is_ok()
        );
        for (method, path) in [
            (Method::POST, "/api/s/default/rest/device"),
            (Method::PUT, "/api/s/default/stat/device"),
            (Method::POST, "/api/auth/login/"),
            (Method::POST, "/proxy/network/api/auth/login"),
            (Method::GET, "//attacker.invalid/api/s/default/stat/device"),
        ] {
            assert!(
                validate_request(&method, path).is_err(),
                "accepted {method} {path}"
            );
        }
    }

    #[test]
    fn endpoint_validation_pins_dns_identity_and_scopes_tls_exception() {
        assert!(validated_origin(&credential("secret")).is_ok());
        let mut invalid = credential("secret");
        invalid.server_name = "other.example.com".to_string();
        assert!(validated_origin(&invalid).is_err());
        invalid = credential("secret");
        invalid.controller_url = "http://controller.example.com".to_string();
        assert!(validated_origin(&invalid).is_err());
        invalid = credential("secret");
        invalid.site = "../other".to_string();
        assert!(validated_origin(&invalid).is_err());
        assert_eq!(credential("secret").tls_policy, UnifiTlsPolicy::Verify);
    }

    #[tokio::test]
    async fn authentication_errors_do_not_expose_credentials() {
        let transport = Arc::new(MockTransport {
            responses: Mutex::new(VecDeque::from([response(
                401,
                json!({"message": "rejected"}),
            )])),
            requests: Mutex::new(Vec::new()),
        });
        let mut client = UnifiClient::new(credential("synthetic-top-secret"), transport);
        let error = client.login().await.unwrap_err().to_string();
        assert!(!error.contains("synthetic-top-secret"));
        assert!(error.contains("rejected"));
    }

    #[test]
    fn file_resolution_failure_does_not_expose_the_local_path() {
        let sentinel = "C:/sentinel/private/controller-password";
        let mut value = credential("unused");
        value.password = ResolvableSecret::FilePath {
            path: sentinel.to_string(),
        };
        let payload = CredentialQueryPayload::Unifi(value);
        let error = resolve_unifi_credential(&payload, "192.0.2.1".parse().unwrap())
            .unwrap_err()
            .message;
        assert!(!error.contains(sentinel));
    }

    #[test]
    fn normalization_is_bounded_and_maps_interfaces_without_synthetic_uplinks() {
        let devices = normalize_devices(&[
            json!({
                "_id": "switch-1",
                "name": "Fixture Switch",
                "ip": "192.0.2.10",
                "mac": "02:00:00:00:00:01",
                "port_table": [{"port_idx": 1, "name": "Port 1", "up": true, "speed": 1000}]
            }),
            json!({
                "_id": "ap-1",
                "name": "Fixture AP",
                "ip": "192.0.2.11",
                "uplink_device": "switch-1"
            }),
        ])
        .unwrap();
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].ports[0].speed_bps, Some(1_000_000_000));
        assert!(device_interfaces(&devices[1], uuid::Uuid::nil()).is_empty());
    }

    #[test]
    fn controller_interfaces_enrich_without_duplicates_or_completeness_loss() {
        let host = Host::new(HostBase::default());
        let mut existing = Interface::new(InterfaceBase {
            if_index: 1,
            if_descr: "SNMP port".into(),
            if_name: Some("eth1".into()),
            ..Default::default()
        });
        existing.base.speed_bps = None;
        let mut host_data = HostData::new(host, vec![], vec![], vec![], vec![existing], vec![]);
        let controller = Interface::new(InterfaceBase {
            if_index: 1,
            if_descr: "Controller port".into(),
            if_name: Some("ETH1".into()),
            speed_bps: Some(1_000_000_000),
            ..Default::default()
        });

        merge_controller_interfaces(&mut host_data, vec![controller]);

        assert_eq!(host_data.interfaces.len(), 1);
        assert_eq!(host_data.interfaces[0].base.if_descr, "SNMP port");
        assert_eq!(host_data.interfaces[0].base.speed_bps, Some(1_000_000_000));
        assert!(host_data.interfaces_complete);
    }

    #[test]
    fn controller_only_interfaces_are_partial() {
        let host = Host::new(HostBase::default());
        let mut host_data = HostData::new(host, vec![], vec![], vec![], vec![], vec![]);
        let controller = Interface::new(InterfaceBase {
            if_index: 1,
            if_descr: "Controller port".into(),
            ..Default::default()
        });

        merge_controller_interfaces(&mut host_data, vec![controller]);

        assert_eq!(host_data.interfaces.len(), 1);
        assert!(!host_data.interfaces_complete);
    }

    #[test]
    fn controller_interface_conflicts_never_cross_enrich_or_double_claim() {
        let host = Host::new(HostBase::default());
        let existing = Interface::new(InterfaceBase {
            if_index: 7,
            if_descr: "SNMP port".into(),
            if_name: Some("eth7".into()),
            ..Default::default()
        });
        let mut host_data = HostData::new(host, vec![], vec![], vec![], vec![existing], vec![]);
        let reused_index = Interface::new(InterfaceBase {
            if_index: 7,
            if_descr: "conflict".into(),
            if_name: Some("other7".into()),
            speed_bps: Some(10),
            ..Default::default()
        });
        let reused_name = Interface::new(InterfaceBase {
            if_index: 8,
            if_descr: "conflict".into(),
            if_name: Some("eth7".into()),
            speed_bps: Some(20),
            ..Default::default()
        });
        let exact = Interface::new(InterfaceBase {
            if_index: 7,
            if_descr: "exact".into(),
            if_name: Some("ETH7".into()),
            speed_bps: Some(30),
            ..Default::default()
        });
        let duplicate_exact = Interface::new(InterfaceBase {
            if_index: 7,
            if_descr: "duplicate".into(),
            if_name: Some("eth7".into()),
            speed_bps: Some(40),
            ..Default::default()
        });

        merge_controller_interfaces(
            &mut host_data,
            vec![reused_index, reused_name, exact, duplicate_exact],
        );

        assert_eq!(host_data.interfaces.len(), 1);
        assert_eq!(host_data.interfaces[0].base.speed_bps, Some(30));
        assert!(host_data.interfaces_complete);
    }
}

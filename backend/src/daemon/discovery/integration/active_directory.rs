//! Bounded, read-only Active Directory collection over LDAPS.
//!
//! The collector requests only documented inventory attributes. It never follows
//! referrals, never permits plaintext LDAP or disabled certificate validation,
//! and never retains or transmits raw LDAP entries. Results are normalized into
//! the server's strict AD collection DTO before authenticated persistence.

use std::{
    collections::HashMap,
    future::Future,
    io::BufReader,
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use anyhow::{Error, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ldap3::{
    Ldap, LdapConnAsync, LdapConnSettings, Scope, SearchEntry, SearchOptions, StdStream,
    ldap_escape,
};
use rustls::{ClientConfig, RootCertStore};
use tokio::{sync::Mutex, task::JoinHandle};
use tokio_util::sync::CancellationToken;

#[cfg(any(all(feature = "ad-gssapi", unix), all(test, unix)))]
use std::path::{Path, PathBuf};

use crate::server::{
    active_directory::types::{
        AdCollectedDomain, AdCollectedEntity, AdCollectionIssue, AdCollectionRequest,
        AdCollectionStatus, AdEntityKind,
    },
    credentials::r#impl::mapping::{
        ActiveDirectoryKerberosQueryCredential, ActiveDirectoryLdapsQueryCredential,
        CredentialQueryPayload, CredentialQueryPayloadDiscriminants, ResolvableSecret,
        ResolvableValue,
    },
    ports::r#impl::base::PortType,
    services::r#impl::patterns::ClientProbe,
};

use super::{
    DiscoveryIntegration, IntegrationContext, IntegrationFailure, ProbeContext, ProbeFailure,
    ProbeSuccess,
};
use crate::daemon::discovery::service::ops::HostData;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const OPERATION_TIMEOUT: Duration = Duration::from_secs(20);
const TOTAL_TIMEOUT: Duration = Duration::from_secs(90);
const SEARCH_TIME_LIMIT_SECONDS: i32 = 15;
const MAX_COMPUTERS: usize = 1_000;
const MAX_SITES: usize = 256;
const MAX_SUBNETS: usize = 512;
const MAX_CONTROLLERS: usize = 128;
const MAX_TRUSTS: usize = 128;
const MAX_GROUPS: usize = 16;
const MAX_GROUP_MEMBERS: usize = 100;
const MAX_LDAP_ENTRY_BYTES: usize = 64 * 1024;

pub struct ActiveDirectoryLdapsIntegration;
pub struct ActiveDirectoryKerberosIntegration;

#[derive(Clone, Copy)]
struct AdReadScope<'a> {
    base_dn: &'a str,
    group_dns: Option<&'a str>,
}

struct AdProbeHandle {
    ldap: Mutex<Ldap>,
    connection_task: JoinHandle<()>,
}

impl Drop for AdProbeHandle {
    fn drop(&mut self) {
        self.connection_task.abort();
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
struct AdInventory {
    domain: Option<AdDomain>,
    sites: Vec<AdNamedObject>,
    subnets: Vec<AdSubnet>,
    controllers: Vec<AdComputer>,
    computers: Vec<AdComputer>,
    trusts: Vec<AdTrust>,
    groups: Vec<AdGroup>,
    truncated: bool,
}

#[derive(Debug, PartialEq, Eq)]
struct AdDomain {
    dns_name: String,
    forest_dns_name: Option<String>,
    functional_level: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
struct AdNamedObject {
    distinguished_name: String,
    object_guid: Option<uuid::Uuid>,
    name: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
struct AdSubnet {
    distinguished_name: String,
    object_guid: Option<uuid::Uuid>,
    name: Option<String>,
    site_dn: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
struct AdComputer {
    distinguished_name: String,
    object_guid: Option<uuid::Uuid>,
    name: Option<String>,
    dns_hostname: Option<String>,
    site_name: Option<String>,
    operating_system: Option<String>,
    operating_system_version: Option<String>,
    is_enabled: Option<bool>,
}

#[derive(Debug, PartialEq, Eq)]
struct AdTrust {
    distinguished_name: String,
    object_guid: Option<uuid::Uuid>,
    partner: Option<String>,
    flat_name: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
struct AdGroup {
    distinguished_name: String,
    object_guid: Option<uuid::Uuid>,
    name: Option<String>,
    member_guids: Vec<uuid::Uuid>,
}

#[derive(Debug)]
struct SearchBatch {
    entries: Vec<SearchEntry>,
    truncated: bool,
}

#[derive(Clone, Copy)]
struct AdCollectionIdentity {
    network_id: uuid::Uuid,
    credential_id: uuid::Uuid,
    target_host_id: uuid::Uuid,
    target_ip: IpAddr,
    discovery_id: uuid::Uuid,
    session_id: uuid::Uuid,
}

#[async_trait]
impl DiscoveryIntegration for ActiveDirectoryLdapsIntegration {
    fn credential_type(&self) -> CredentialQueryPayloadDiscriminants {
        CredentialQueryPayloadDiscriminants::ActiveDirectoryLdaps
    }

    fn estimated_seconds(&self) -> u32 {
        30
    }

    fn timeout(&self) -> Duration {
        TOTAL_TIMEOUT
    }

    fn probe_gate_ports(&self, credential: &CredentialQueryPayload) -> Vec<PortType> {
        match credential {
            CredentialQueryPayload::ActiveDirectoryLdaps(ad) => vec![port_type(ad.port)],
            _ => Vec::new(),
        }
    }

    async fn probe(&self, ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure> {
        let raw = match ctx.credential {
            CredentialQueryPayload::ActiveDirectoryLdaps(credential) => credential,
            _ => return Err(failure("expected Active Directory LDAPS credential")),
        };
        let resolved = ctx.credential.resolve_file_paths().map_err(|error| {
            sanitized_failure("credential material could not be resolved", error)
        })?;
        let CredentialQueryPayload::ActiveDirectoryLdaps(credential) = resolved else {
            return Err(failure("expected Active Directory LDAPS credential"));
        };

        // Keep the raw value referenced so a future wire change cannot silently
        // bypass the type check above.
        debug_assert_eq!(raw.port, credential.port);
        let handle = connect_and_bind(ctx.ip, &credential, ctx.cancel)
            .await
            .map_err(|error| sanitized_failure("LDAPS connection or bind failed", error))?;

        Ok(ProbeSuccess {
            client_probe: ClientProbe::ActiveDirectory,
            ports: vec![port_type(credential.port)],
            handle: Some(Box::new(handle)),
        })
    }

    async fn execute(
        &self,
        ctx: &IntegrationContext<'_>,
        _host_data: &mut HostData,
    ) -> Result<(), IntegrationFailure> {
        let credential = match ctx.credential {
            CredentialQueryPayload::ActiveDirectoryLdaps(credential) => credential,
            _ => return Err(anyhow!("expected Active Directory LDAPS credential").into()),
        };
        execute_collection(
            ctx,
            AdReadScope {
                base_dn: &credential.base_dn,
                group_dns: credential.group_dns.as_deref(),
            },
        )
        .await
        .map_err(Into::into)
    }
}

#[async_trait]
impl DiscoveryIntegration for ActiveDirectoryKerberosIntegration {
    fn credential_type(&self) -> CredentialQueryPayloadDiscriminants {
        CredentialQueryPayloadDiscriminants::ActiveDirectoryKerberos
    }

    fn estimated_seconds(&self) -> u32 {
        30
    }

    fn timeout(&self) -> Duration {
        TOTAL_TIMEOUT
    }

    fn probe_gate_ports(&self, credential: &CredentialQueryPayload) -> Vec<PortType> {
        match credential {
            CredentialQueryPayload::ActiveDirectoryKerberos(ad) => vec![port_type(ad.port)],
            _ => Vec::new(),
        }
    }

    async fn probe(&self, ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure> {
        let raw = match ctx.credential {
            CredentialQueryPayload::ActiveDirectoryKerberos(credential) => credential,
            _ => return Err(failure("expected Active Directory Kerberos credential")),
        };
        let resolved = ctx.credential.resolve_file_paths().map_err(|error| {
            sanitized_failure("credential material could not be resolved", error)
        })?;
        let CredentialQueryPayload::ActiveDirectoryKerberos(credential) = resolved else {
            return Err(failure("expected Active Directory Kerberos credential"));
        };
        debug_assert_eq!(raw.port, credential.port);
        let handle = connect_and_bind_kerberos(ctx.ip, &credential, ctx.cancel)
            .await
            .map_err(|error| {
                sanitized_failure("Kerberos LDAPS connection or bind failed", error)
            })?;

        Ok(ProbeSuccess {
            client_probe: ClientProbe::ActiveDirectory,
            ports: vec![port_type(credential.port)],
            handle: Some(Box::new(handle)),
        })
    }

    async fn execute(
        &self,
        ctx: &IntegrationContext<'_>,
        _host_data: &mut HostData,
    ) -> Result<(), IntegrationFailure> {
        let credential = match ctx.credential {
            CredentialQueryPayload::ActiveDirectoryKerberos(credential) => credential,
            _ => return Err(anyhow!("expected Active Directory Kerberos credential").into()),
        };
        execute_collection(
            ctx,
            AdReadScope {
                base_dn: &credential.base_dn,
                group_dns: credential.group_dns.as_deref(),
            },
        )
        .await
        .map_err(Into::into)
    }
}

async fn execute_collection(
    ctx: &IntegrationContext<'_>,
    scope: AdReadScope<'_>,
) -> Result<(), Error> {
    let handle = ctx
        .probe_handle
        .and_then(|value| value.downcast_ref::<AdProbeHandle>())
        .ok_or_else(|| anyhow!("Active Directory execute called without a probe handle"))?;
    let started_at = Utc::now();
    let session = ctx.ops.get_session().await?;
    let identity = AdCollectionIdentity {
        network_id: session.info.network_id,
        credential_id: ctx
            .credential_id
            .ok_or_else(|| anyhow!("Active Directory credential ID is missing"))?,
        target_host_id: ctx.host_id,
        target_ip: ctx.ip,
        discovery_id: session.info.discovery_id,
        session_id: session.info.session_id,
    };
    let collection_result = async {
        let mut ldap = handle.ldap.lock().await;
        let inventory = collect_inventory(&mut ldap, scope, ctx.cancel).await?;
        normalize_collection(inventory, identity, started_at, Utc::now())
    }
    .await;
    let request = match collection_result {
        Ok(request) => request,
        Err(error) => {
            let failed_request = failed_collection(identity, started_at, Utc::now());
            if let Err(report_error) = ctx
                .ops
                .api_client
                .post_no_data(
                    "/api/v1/active-directory/collection-runs",
                    &failed_request,
                    "Failed to persist Active Directory collection failure",
                )
                .await
            {
                tracing::warn!(error = %report_error, "Could not persist AD collection failure");
            }
            return Err(error);
        }
    };
    request
        .validate()
        .map_err(|error| anyhow!("normalized Active Directory collection is invalid: {error}"))?;
    controlled(
        ctx.cancel,
        OPERATION_TIMEOUT,
        ctx.ops.api_client.post_no_data(
            "/api/v1/active-directory/collection-runs",
            &request,
            "Failed to persist Active Directory collection",
        ),
    )
    .await??;
    tracing::info!(ip = %ctx.ip, "Persisted bounded Active Directory inventory collection");
    Ok(())
}

fn failed_collection(
    identity: AdCollectionIdentity,
    started_at: DateTime<Utc>,
    completed_at: DateTime<Utc>,
) -> AdCollectionRequest {
    AdCollectionRequest {
        network_id: identity.network_id,
        credential_id: identity.credential_id,
        target_host_id: identity.target_host_id,
        target_ip: identity.target_ip,
        discovery_id: identity.discovery_id,
        session_id: identity.session_id,
        status: AdCollectionStatus::Failed,
        started_at,
        completed_at,
        truncated: true,
        issues: vec![AdCollectionIssue {
            code: "collector_failure".to_string(),
            message: "Directory collection failed before a complete inventory was produced."
                .to_string(),
            entity_external_id: None,
        }],
        domains: Vec::new(),
    }
}

async fn connect_and_bind(
    ip: IpAddr,
    credential: &ActiveDirectoryLdapsQueryCredential,
    cancel: &CancellationToken,
) -> Result<AdProbeHandle, Error> {
    let (mut ldap, connection_task) = connect_tls(
        ip,
        credential.port,
        &credential.server_name,
        credential.ca_certificate.as_ref(),
        cancel,
    )
    .await?;
    let password = resolved_secret(&credential.password)?;
    let bind = controlled(
        cancel,
        OPERATION_TIMEOUT,
        ldap.simple_bind(&credential.bind_dn, password),
    )
    .await;
    finish_bind(bind, &connection_task)?;

    Ok(AdProbeHandle {
        ldap: Mutex::new(ldap),
        connection_task,
    })
}

async fn connect_tls(
    ip: IpAddr,
    port: u16,
    server_name: &str,
    ca_certificate: Option<&ResolvableValue>,
    cancel: &CancellationToken,
) -> Result<(Ldap, JoinHandle<()>), Error> {
    let address = SocketAddr::new(ip, port);
    let stream = controlled(
        cancel,
        CONNECT_TIMEOUT,
        tokio::net::TcpStream::connect(address),
    )
    .await??;
    let std_stream = stream.into_std()?;
    let settings = tls_settings(ca_certificate)?
        .set_conn_timeout(CONNECT_TIMEOUT)
        .set_std_stream(StdStream::Tcp(std_stream));
    let url = ldaps_url(server_name, port)?;
    let (connection, ldap) = controlled(
        cancel,
        CONNECT_TIMEOUT,
        LdapConnAsync::with_settings(settings, &url),
    )
    .await??;
    let connection_task = tokio::spawn(async move {
        if let Err(error) = connection.drive().await {
            tracing::debug!(error = %error, "LDAPS connection driver stopped");
        }
    });

    Ok((ldap, connection_task))
}

fn finish_bind(
    bind: Result<Result<ldap3::LdapResult, ldap3::LdapError>, Error>,
    connection_task: &JoinHandle<()>,
) -> Result<(), Error> {
    match bind {
        Ok(Ok(result)) => {
            if let Err(error) = result.success() {
                connection_task.abort();
                return Err(error.into());
            }
        }
        Ok(Err(error)) => {
            connection_task.abort();
            return Err(error.into());
        }
        Err(error) => {
            connection_task.abort();
            return Err(error);
        }
    }
    Ok(())
}

#[cfg(all(feature = "ad-gssapi", unix))]
async fn connect_and_bind_kerberos(
    ip: IpAddr,
    credential: &ActiveDirectoryKerberosQueryCredential,
    cancel: &CancellationToken,
) -> Result<AdProbeHandle, Error> {
    use cross_krb5::{Cred, InitiateFlags, K5Cred};

    if !credential.use_system_ccache {
        return Err(anyhow!(
            "system credential cache acknowledgement is missing"
        ));
    }
    verify_read_only_system_ccache()?;

    // Credential acquisition can perform blocking GSSAPI filesystem work. Keep
    // it off the async executor and require the exact configured principal; the
    // default-principal helper is deliberately never called.
    let principal = credential.principal.clone();
    let cred = controlled(cancel, OPERATION_TIMEOUT, async move {
        tokio::task::spawn_blocking(move || {
            Cred::client_acquire(InitiateFlags::empty(), Some(&principal))
        })
        .await
    })
    .await??
    .map_err(|_| anyhow!("configured Kerberos principal is unavailable"))?;

    let (mut ldap, connection_task) = connect_tls(
        ip,
        credential.port,
        &credential.server_name,
        credential.ca_certificate.as_ref(),
        cancel,
    )
    .await?;
    let bind = controlled(
        cancel,
        OPERATION_TIMEOUT,
        ldap.sasl_gssapi_cred_bind(cred, &credential.server_name),
    )
    .await;
    finish_bind(bind, &connection_task)?;

    Ok(AdProbeHandle {
        ldap: Mutex::new(ldap),
        connection_task,
    })
}

#[cfg(not(all(feature = "ad-gssapi", unix)))]
async fn connect_and_bind_kerberos(
    _ip: IpAddr,
    _credential: &ActiveDirectoryKerberosQueryCredential,
    _cancel: &CancellationToken,
) -> Result<AdProbeHandle, Error> {
    Err(anyhow!(
        "Kerberos system ccache support is unavailable in this daemon build"
    ))
}

#[cfg(all(feature = "ad-gssapi", unix))]
fn verify_read_only_system_ccache() -> Result<(), Error> {
    let configured = std::env::var("KRB5CCNAME")
        .map_err(|_| anyhow!("system credential cache is not configured"))?;
    let path = system_ccache_path(&configured)?;
    let metadata =
        std::fs::metadata(&path).map_err(|_| anyhow!("system credential cache is unavailable"))?;
    if !metadata.is_file() {
        return Err(anyhow!("system credential cache is unavailable"));
    }
    std::fs::OpenOptions::new()
        .read(true)
        .open(&path)
        .map_err(|_| anyhow!("system credential cache is unreadable"))?;
    if std::fs::OpenOptions::new().write(true).open(&path).is_ok() {
        return Err(anyhow!(
            "system credential cache must be mounted or permissioned read-only"
        ));
    }
    Ok(())
}

#[cfg(any(all(feature = "ad-gssapi", unix), all(test, unix)))]
fn system_ccache_path(value: &str) -> Result<PathBuf, Error> {
    let path = value
        .strip_prefix("FILE:")
        .ok_or_else(|| anyhow!("system credential cache must use the FILE cache type"))?;
    if path.is_empty() || !Path::new(path).is_absolute() {
        return Err(anyhow!(
            "system credential cache must use one absolute FILE path"
        ));
    }
    Ok(PathBuf::from(path))
}

fn tls_settings(ca_certificate: Option<&ResolvableValue>) -> Result<LdapConnSettings, Error> {
    let mut settings = LdapConnSettings::new();
    if let Some(certificate) = ca_certificate {
        let pem = resolved_value(certificate)?;
        let mut reader = BufReader::new(pem.as_bytes());
        let certificates = rustls_pemfile::certs(&mut reader).collect::<Result<Vec<_>, _>>()?;
        if certificates.is_empty() {
            return Err(anyhow!("custom LDAPS CA contains no certificates"));
        }
        let mut roots = RootCertStore::empty();
        let (added, rejected) = roots.add_parsable_certificates(certificates);
        if added == 0 || rejected != 0 {
            return Err(anyhow!("custom LDAPS CA contains an invalid certificate"));
        }
        let config = ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        settings = settings.set_config(Arc::new(config));
    }
    // Deliberately do not call set_no_tls_verify. Verification is mandatory.
    Ok(settings)
}

fn ldaps_url(server_name: &str, port: u16) -> Result<String, Error> {
    if server_name.is_empty() || server_name.contains("://") || server_name.contains('/') {
        return Err(anyhow!("invalid LDAPS server name"));
    }
    Ok(format!("ldaps://{server_name}:{port}"))
}

async fn collect_inventory(
    ldap: &mut Ldap,
    read_scope: AdReadScope<'_>,
    cancel: &CancellationToken,
) -> Result<AdInventory, Error> {
    let root_batch = search(
        ldap,
        cancel,
        "",
        Scope::Base,
        "(objectClass=*)",
        &["configurationNamingContext", "rootDomainNamingContext"],
        1,
    )
    .await?;
    let mut truncated = root_batch.truncated;
    let root_entry = root_batch.entries.into_iter().next();
    if root_entry.is_none() {
        truncated = true;
    }
    let forest_dns_name = root_entry
        .as_ref()
        .and_then(|entry| first_attr(entry, "rootDomainNamingContext"))
        .and_then(|dn| dns_name_from_dn(&dn));
    let config_dn = root_entry
        .as_ref()
        .and_then(|entry| first_attr(entry, "configurationNamingContext"));
    if config_dn.is_none() {
        truncated = true;
    }

    let domain_batch = search(
        ldap,
        cancel,
        read_scope.base_dn,
        Scope::Base,
        "(objectClass=domainDNS)",
        &["name", "distinguishedName", "msDS-Behavior-Version"],
        1,
    )
    .await?;
    truncated |= domain_batch.truncated;
    let domain = domain_batch
        .entries
        .into_iter()
        .next()
        .map(|entry| parse_domain(entry, forest_dns_name));

    let (sites, subnets) = if let Some(config_dn) = config_dn {
        let site_batch = search(
            ldap,
            cancel,
            &config_dn,
            Scope::Subtree,
            "(objectClass=site)",
            &["name", "distinguishedName", "objectGUID"],
            MAX_SITES,
        )
        .await?;
        truncated |= site_batch.truncated;
        let sites = site_batch.entries.into_iter().map(parse_named).collect();
        let subnet_batch = search(
            ldap,
            cancel,
            &config_dn,
            Scope::Subtree,
            "(objectClass=subnet)",
            &["name", "siteObject", "distinguishedName", "objectGUID"],
            MAX_SUBNETS,
        )
        .await?;
        truncated |= subnet_batch.truncated;
        let subnets = subnet_batch.entries.into_iter().map(parse_subnet).collect();
        (sites, subnets)
    } else {
        (Vec::new(), Vec::new())
    };

    let computer_attributes = [
        "name",
        "dNSHostName",
        "msDS-SiteName",
        "operatingSystem",
        "operatingSystemVersion",
        "userAccountControl",
        "distinguishedName",
        "objectGUID",
    ];
    let controller_batch = search(
        ldap,
        cancel,
        read_scope.base_dn,
        Scope::Subtree,
        "(&(objectCategory=computer)(userAccountControl:1.2.840.113556.1.4.803:=8192))",
        &computer_attributes,
        MAX_CONTROLLERS,
    )
    .await?;
    truncated |= controller_batch.truncated;
    let controllers = controller_batch
        .entries
        .into_iter()
        .map(parse_computer)
        .collect();
    let computer_batch = search(
        ldap,
        cancel,
        read_scope.base_dn,
        Scope::Subtree,
        "(objectCategory=computer)",
        &computer_attributes,
        MAX_COMPUTERS,
    )
    .await?;
    truncated |= computer_batch.truncated;
    let computers = computer_batch
        .entries
        .into_iter()
        .map(parse_computer)
        .collect();
    let trust_batch = search(
        ldap,
        cancel,
        read_scope.base_dn,
        Scope::Subtree,
        "(objectClass=trustedDomain)",
        &[
            "trustPartner",
            "flatName",
            "distinguishedName",
            "objectGUID",
        ],
        MAX_TRUSTS,
    )
    .await?;
    truncated |= trust_batch.truncated;
    let trusts = trust_batch.entries.into_iter().map(parse_trust).collect();

    let mut groups = Vec::new();
    for group_dn in configured_group_dns(read_scope.group_dns)? {
        let group_batch = search(
            ldap,
            cancel,
            group_dn,
            Scope::Base,
            "(objectClass=group)",
            &["name", "distinguishedName", "objectGUID"],
            1,
        )
        .await?;
        truncated |= group_batch.truncated;
        if let Some(entry) = group_batch.entries.into_iter().next() {
            let filter = format!("(memberOf={})", ldap_escape(group_dn));
            let member_batch = search(
                ldap,
                cancel,
                read_scope.base_dn,
                Scope::Subtree,
                &filter,
                &["objectGUID"],
                MAX_GROUP_MEMBERS,
            )
            .await?;
            truncated |= member_batch.truncated;
            let member_guids = member_batch
                .entries
                .iter()
                .filter_map(object_guid)
                .collect::<Vec<_>>();
            if member_guids.len() != member_batch.entries.len() {
                truncated = true;
            }
            groups.push(parse_group(entry, member_guids));
        } else {
            truncated = true;
        }
    }

    Ok(AdInventory {
        domain,
        sites,
        subnets,
        controllers,
        computers,
        trusts,
        groups,
        truncated,
    })
}

async fn search(
    ldap: &mut Ldap,
    cancel: &CancellationToken,
    base: &str,
    scope: Scope,
    filter: &str,
    attributes: &[&str],
    limit: usize,
) -> Result<SearchBatch, Error> {
    let limit = limit.min(i32::MAX as usize);
    let server_limit = limit.saturating_add(1).min(i32::MAX as usize);
    ldap.with_search_options(
        SearchOptions::new()
            .timelimit(SEARCH_TIME_LIMIT_SECONDS)
            .sizelimit(server_limit as i32),
    );
    let mut stream = controlled(
        cancel,
        OPERATION_TIMEOUT,
        ldap.streaming_search(base, scope, filter, attributes),
    )
    .await??;

    let mut entries = Vec::with_capacity(limit);
    let mut truncated = false;
    loop {
        let next = controlled(cancel, OPERATION_TIMEOUT, stream.next()).await??;
        let Some(raw_entry) = next else { break };
        if entries.len() == limit {
            truncated = true;
            break;
        }
        let entry = std::panic::catch_unwind(|| SearchEntry::construct(raw_entry))
            .map_err(|_| anyhow!("LDAP search returned a malformed entry"))?;
        if entry_size(&entry) > MAX_LDAP_ENTRY_BYTES {
            truncated = true;
            continue;
        }
        entries.push(entry);
    }
    let result = controlled(cancel, OPERATION_TIMEOUT, stream.finish()).await?;
    if result.rc == 4 || result.rc == 11 {
        truncated = true;
    } else if result.rc != 0 && !(truncated && result.rc == 88) {
        return Err(anyhow!("LDAP search failed with result code {}", result.rc));
    }
    Ok(SearchBatch { entries, truncated })
}

fn entry_size(entry: &SearchEntry) -> usize {
    entry.dn.len()
        + entry
            .attrs
            .iter()
            .map(|(name, values)| name.len() + values.iter().map(String::len).sum::<usize>())
            .sum::<usize>()
        + entry
            .bin_attrs
            .iter()
            .map(|(name, values)| name.len() + values.iter().map(Vec::len).sum::<usize>())
            .sum::<usize>()
}

fn normalize_collection(
    inventory: AdInventory,
    identity: AdCollectionIdentity,
    started_at: DateTime<Utc>,
    completed_at: DateTime<Utc>,
) -> Result<AdCollectionRequest, Error> {
    let domain = inventory
        .domain
        .ok_or_else(|| anyhow!("Active Directory domain object was not returned"))?;
    if domain.dns_name.is_empty() {
        return Err(anyhow!(
            "Active Directory base DN does not contain a DNS domain"
        ));
    }

    let mut truncated = inventory.truncated;
    let mut entities = Vec::new();
    let observed_at = completed_at;
    let site_ids = inventory
        .sites
        .iter()
        .filter_map(|site| {
            site.object_guid.map(|guid| {
                (
                    site.distinguished_name.to_ascii_lowercase(),
                    guid.to_string(),
                )
            })
        })
        .collect::<HashMap<_, _>>();

    for site in inventory.sites {
        if let Some(entity) = basic_entity(
            AdEntityKind::Site,
            site.object_guid,
            &site.distinguished_name,
            site.name.as_deref(),
            observed_at,
        ) {
            entities.push(entity);
        } else {
            truncated = true;
        }
    }
    for subnet in inventory.subnets {
        let Some(network_prefix) = subnet
            .name
            .as_deref()
            .filter(|value| value.parse::<ipnetwork::IpNetwork>().is_ok())
            .map(str::to_string)
        else {
            truncated = true;
            continue;
        };
        let Some(mut entity) = basic_entity(
            AdEntityKind::Subnet,
            subnet.object_guid,
            &subnet.distinguished_name,
            subnet.name.as_deref(),
            observed_at,
        ) else {
            truncated = true;
            continue;
        };
        entity.network_prefix = Some(network_prefix);
        entity.parent_external_id = subnet
            .site_dn
            .as_deref()
            .and_then(|dn| site_ids.get(&dn.to_ascii_lowercase()))
            .cloned();
        if subnet.site_dn.is_some() && entity.parent_external_id.is_none() {
            truncated = true;
        }
        entities.push(entity);
    }
    for (kind, computers) in [
        (AdEntityKind::DomainController, inventory.controllers),
        (AdEntityKind::Computer, inventory.computers),
    ] {
        for computer in computers {
            let Some(mut entity) = basic_entity(
                kind,
                computer.object_guid,
                &computer.distinguished_name,
                computer
                    .name
                    .as_deref()
                    .or(computer.dns_hostname.as_deref()),
                observed_at,
            ) else {
                truncated = true;
                continue;
            };
            entity.dns_name = computer
                .dns_hostname
                .as_deref()
                .filter(|value| valid_dns_name(value))
                .map(|value| value.to_ascii_lowercase());
            entity.site_name = bounded_text(computer.site_name.as_deref(), 256);
            entity.operating_system = bounded_text(computer.operating_system.as_deref(), 256);
            entity.operating_system_version =
                bounded_text(computer.operating_system_version.as_deref(), 128);
            entity.is_enabled = computer.is_enabled;
            entities.push(entity);
        }
    }
    for trust in inventory.trusts {
        let Some(partner) = trust
            .partner
            .as_deref()
            .filter(|value| valid_dns_name(value))
        else {
            truncated = true;
            continue;
        };
        let Some(mut entity) = basic_entity(
            AdEntityKind::Trust,
            trust.object_guid,
            &trust.distinguished_name,
            trust.flat_name.as_deref().or(Some(partner)),
            observed_at,
        ) else {
            truncated = true;
            continue;
        };
        entity.related_external_id = Some(
            uuid::Uuid::new_v5(
                &uuid::Uuid::NAMESPACE_DNS,
                partner.to_ascii_lowercase().as_bytes(),
            )
            .to_string(),
        );
        entities.push(entity);
    }
    for group in inventory.groups {
        let Some(group_entity) = basic_entity(
            AdEntityKind::Group,
            group.object_guid,
            &group.distinguished_name,
            group.name.as_deref(),
            observed_at,
        ) else {
            truncated = true;
            continue;
        };
        let group_external_id = group_entity.external_id.clone();
        entities.push(group_entity);
        for member_guid in group.member_guids {
            let member_external_id = member_guid.to_string();
            let relation_key = format!("{group_external_id}\0{member_external_id}");
            entities.push(AdCollectedEntity {
                kind: AdEntityKind::GroupMembership,
                external_id: uuid::Uuid::new_v5(
                    &uuid::Uuid::NAMESPACE_OID,
                    relation_key.as_bytes(),
                )
                .to_string(),
                name: "Configured group member".to_string(),
                dns_name: None,
                parent_external_id: Some(group_external_id.clone()),
                related_external_id: Some(member_external_id),
                site_name: None,
                operating_system: None,
                operating_system_version: None,
                network_prefix: None,
                is_enabled: None,
                observed_at,
            });
        }
    }

    let issues = if truncated {
        vec![AdCollectionIssue {
            code: "limit_reached".to_string(),
            message: "One or more directory result limits were reached; inventory is partial."
                .to_string(),
            entity_external_id: None,
        }]
    } else {
        Vec::new()
    };
    Ok(AdCollectionRequest {
        network_id: identity.network_id,
        credential_id: identity.credential_id,
        target_host_id: identity.target_host_id,
        target_ip: identity.target_ip,
        discovery_id: identity.discovery_id,
        session_id: identity.session_id,
        status: if truncated {
            AdCollectionStatus::Partial
        } else {
            AdCollectionStatus::Succeeded
        },
        started_at,
        completed_at,
        truncated,
        issues,
        domains: vec![AdCollectedDomain {
            dns_name: domain.dns_name.to_ascii_lowercase(),
            forest_dns_name: domain.forest_dns_name.map(|name| name.to_ascii_lowercase()),
            netbios_name: None,
            functional_level: bounded_text(domain.functional_level.as_deref(), 100),
            observed_at,
            entities,
        }],
    })
}

fn basic_entity(
    kind: AdEntityKind,
    object_guid: Option<uuid::Uuid>,
    distinguished_name: &str,
    name: Option<&str>,
    observed_at: DateTime<Utc>,
) -> Option<AdCollectedEntity> {
    Some(AdCollectedEntity {
        kind,
        external_id: object_guid?.to_string(),
        name: {
            let candidate = name
                .map(str::to_string)
                .unwrap_or_else(|| rdn_name(distinguished_name));
            bounded_text(Some(&candidate), 256)?
        },
        dns_name: None,
        parent_external_id: None,
        related_external_id: None,
        site_name: None,
        operating_system: None,
        operating_system_version: None,
        network_prefix: None,
        is_enabled: None,
        observed_at,
    })
}

fn bounded_text(value: Option<&str>, max_chars: usize) -> Option<String> {
    let value = value?.trim();
    if value.is_empty() || value.chars().any(char::is_control) {
        return None;
    }
    Some(value.chars().take(max_chars).collect())
}

fn rdn_name(dn: &str) -> String {
    dn.split(',')
        .next()
        .and_then(|rdn| rdn.split_once('=').map(|(_, value)| value))
        .unwrap_or(dn)
        .trim()
        .to_string()
}

fn dns_name_from_dn(dn: &str) -> Option<String> {
    let labels = dn
        .split(',')
        .filter_map(|rdn| {
            let (attribute, value) = rdn.trim().split_once('=')?;
            attribute.eq_ignore_ascii_case("DC").then(|| value.trim())
        })
        .collect::<Vec<_>>();
    let value = labels.join(".");
    valid_dns_name(&value).then_some(value)
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

fn configured_group_dns(value: Option<&str>) -> Result<Vec<&str>, Error> {
    let groups = value
        .into_iter()
        .flat_map(str::lines)
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if groups.len() > MAX_GROUPS {
        return Err(anyhow!("too many configured Active Directory groups"));
    }
    Ok(groups)
}

fn parse_domain(entry: SearchEntry, forest_dns_name: Option<String>) -> AdDomain {
    let distinguished_name = entry_dn(&entry);
    AdDomain {
        dns_name: dns_name_from_dn(&distinguished_name).unwrap_or_default(),
        forest_dns_name,
        functional_level: first_attr(&entry, "msDS-Behavior-Version"),
    }
}

fn parse_named(entry: SearchEntry) -> AdNamedObject {
    AdNamedObject {
        distinguished_name: entry_dn(&entry),
        object_guid: object_guid(&entry),
        name: first_attr(&entry, "name"),
    }
}

fn parse_subnet(entry: SearchEntry) -> AdSubnet {
    AdSubnet {
        distinguished_name: entry_dn(&entry),
        object_guid: object_guid(&entry),
        name: first_attr(&entry, "name"),
        site_dn: first_attr(&entry, "siteObject"),
    }
}

fn parse_computer(entry: SearchEntry) -> AdComputer {
    let account_control =
        first_attr(&entry, "userAccountControl").and_then(|value| value.parse::<u32>().ok());
    AdComputer {
        distinguished_name: entry_dn(&entry),
        object_guid: object_guid(&entry),
        name: first_attr(&entry, "name"),
        dns_hostname: first_attr(&entry, "dNSHostName"),
        site_name: first_attr(&entry, "msDS-SiteName"),
        operating_system: first_attr(&entry, "operatingSystem"),
        operating_system_version: first_attr(&entry, "operatingSystemVersion"),
        is_enabled: account_control.map(|flags| flags & 0x2 == 0),
    }
}

fn parse_trust(entry: SearchEntry) -> AdTrust {
    AdTrust {
        distinguished_name: entry_dn(&entry),
        object_guid: object_guid(&entry),
        partner: first_attr(&entry, "trustPartner"),
        flat_name: first_attr(&entry, "flatName"),
    }
}

fn parse_group(entry: SearchEntry, member_guids: Vec<uuid::Uuid>) -> AdGroup {
    AdGroup {
        distinguished_name: entry_dn(&entry),
        object_guid: object_guid(&entry),
        name: first_attr(&entry, "name"),
        member_guids,
    }
}

fn object_guid(entry: &SearchEntry) -> Option<uuid::Uuid> {
    if let Some(bytes) = entry
        .bin_attrs
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("objectGUID"))
        .and_then(|(_, values)| values.first())
    {
        let bytes: [u8; 16] = bytes.as_slice().try_into().ok()?;
        return Some(uuid::Uuid::from_bytes_le(bytes));
    }
    first_attr(entry, "objectGUID").and_then(|value| uuid::Uuid::parse_str(&value).ok())
}

fn entry_dn(entry: &SearchEntry) -> String {
    first_attr(entry, "distinguishedName").unwrap_or_else(|| entry.dn.clone())
}

fn first_attr(entry: &SearchEntry, name: &str) -> Option<String> {
    entry
        .attrs
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .and_then(|(_, values)| values.first())
        .cloned()
}

fn resolved_secret(secret: &ResolvableSecret) -> Result<&str, Error> {
    match secret {
        ResolvableSecret::Value { value } => Ok(value),
        ResolvableSecret::FilePath { .. } => Err(anyhow!("LDAPS password file was not resolved")),
    }
}

fn resolved_value(value: &ResolvableValue) -> Result<&str, Error> {
    match value {
        ResolvableValue::Value { value } => Ok(value),
        ResolvableValue::FilePath { .. } => Err(anyhow!("LDAPS CA file was not resolved")),
    }
}

fn port_type(port: u16) -> PortType {
    if port == PortType::Ldaps.number() {
        PortType::Ldaps
    } else {
        PortType::new_tcp(port)
    }
}

async fn controlled<F, T>(
    cancel: &CancellationToken,
    timeout: Duration,
    future: F,
) -> Result<T, Error>
where
    F: Future<Output = T>,
{
    tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(anyhow!("Active Directory collection was cancelled")),
        result = tokio::time::timeout(timeout, future) => {
            result.map_err(|_| anyhow!("Active Directory operation timed out"))
        }
    }
}

fn failure(message: impl Into<String>) -> ProbeFailure {
    // Reached only when this integration is dispatched with a credential of the wrong
    // shape — a dispatch bug, not something the remote directory did.
    ProbeFailure::malformed(message)
}

fn sanitized_failure(context: &str, _error: Error) -> ProbeFailure {
    // Resolution and native-library errors can contain daemon-local paths or
    // GSSAPI details. Keep both logs and the API-visible probe failure opaque.
    tracing::debug!("{context}");
    ProbeFailure::rejected(context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::HashMap,
        io::{Cursor, Read, Write},
        net::{Ipv4Addr, TcpListener},
        sync::mpsc,
        thread,
    };

    const LDAP_BIND_SUCCESS: &[u8] = &[
        0x30, 0x0c, // LDAPMessage sequence
        0x02, 0x01, 0x01, // message ID 1
        0x61, 0x07, // BindResponse
        0x0a, 0x01, 0x00, // resultCode: success
        0x04, 0x00, // matchedDN: empty
        0x04, 0x00, // diagnosticMessage: empty
    ];

    struct LoopbackLdapsServer {
        address: SocketAddr,
        root_pem: String,
        observation: mpsc::Receiver<(Option<String>, Vec<u8>)>,
        worker: thread::JoinHandle<()>,
    }

    impl LoopbackLdapsServer {
        fn spawn() -> Self {
            let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
            let address = listener.local_addr().unwrap();
            let (observation_sender, observation) = mpsc::channel();
            let rcgen::CertifiedKey { cert, key_pair } =
                rcgen::generate_simple_self_signed(vec!["dc01.example.test".to_string()]).unwrap();
            let certificate_pem = cert.pem();
            let private_key_pem = key_pair.serialize_pem();
            let root_pem = certificate_pem.clone();
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
                let (socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                socket
                    .set_write_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let connection = rustls::ServerConnection::new(Arc::new(config)).unwrap();
                let mut stream = rustls::StreamOwned::new(connection, socket);
                let mut request = vec![0_u8; 4096];
                let request = match stream.read(&mut request) {
                    Ok(count) => {
                        request.truncate(count);
                        request
                    }
                    // An untrusted-certificate test intentionally aborts the
                    // handshake before an LDAP request is sent.
                    Err(_) => Vec::new(),
                };
                let _ = observation_sender
                    .send((stream.conn.server_name().map(ToString::to_string), request));
                if !stream.conn.is_handshaking() {
                    let _ = stream.write_all(LDAP_BIND_SUCCESS);
                    let _ = stream.flush();
                }
            });
            Self {
                address,
                root_pem,
                observation,
                worker,
            }
        }

        fn finish(self) -> (Option<String>, Vec<u8>) {
            let observation = self
                .observation
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
            self.worker.join().unwrap();
            observation
        }
    }

    fn loopback_credential(
        port: u16,
        ca_certificate: Option<String>,
    ) -> ActiveDirectoryLdapsQueryCredential {
        ActiveDirectoryLdapsQueryCredential {
            bind_dn: "CN=Scanopy,DC=example,DC=test".to_string(),
            password: ResolvableSecret::Value {
                value: "fixture-password".to_string(),
            },
            port,
            server_name: "dc01.example.test".to_string(),
            base_dn: "DC=example,DC=test".to_string(),
            ca_certificate: ca_certificate.map(|value| ResolvableValue::Value { value }),
            group_dns: None,
        }
    }

    fn entry(dn: &str, attrs: &[(&str, Vec<String>)]) -> SearchEntry {
        SearchEntry {
            dn: dn.to_string(),
            attrs: attrs
                .iter()
                .map(|(name, values)| ((*name).to_string(), values.clone()))
                .collect::<HashMap<_, _>>(),
            bin_attrs: HashMap::new(),
        }
    }

    #[test]
    fn group_parser_retains_only_supplied_member_guids() {
        let member_guids = (0..MAX_GROUP_MEMBERS)
            .map(|_| uuid::Uuid::new_v4())
            .collect::<Vec<_>>();
        let parsed = parse_group(
            entry(
                "CN=Readers,DC=example,DC=com",
                &[
                    ("name", vec!["Readers".to_string()]),
                    ("member", vec!["CN=Never retained".to_string()]),
                    ("unicodePwd", vec!["must-not-be-retained".to_string()]),
                    ("ms-Mcs-AdmPwd", vec!["must-not-be-retained".to_string()]),
                ],
            ),
            member_guids.clone(),
        );
        assert_eq!(parsed.member_guids, member_guids);
        assert_eq!(parsed.name.as_deref(), Some("Readers"));
        assert!(!format!("{parsed:?}").contains("must-not-be-retained"));
        assert!(!format!("{parsed:?}").contains("CN=Never retained"));
    }

    #[test]
    fn only_explicit_group_dns_are_accepted_and_bounded() {
        assert_eq!(
            configured_group_dns(Some(" CN=A,DC=x \n\nCN=B,DC=x\n")).unwrap(),
            vec!["CN=A,DC=x", "CN=B,DC=x"]
        );
        let too_many = (0..=MAX_GROUPS)
            .map(|n| format!("CN={n},DC=x"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(configured_group_dns(Some(&too_many)).is_err());
    }

    #[test]
    fn ldaps_url_rejects_schemes_and_paths() {
        assert_eq!(
            ldaps_url("dc01.example.com", 636).unwrap(),
            "ldaps://dc01.example.com:636"
        );
        assert!(ldaps_url("https://dc01.example.com", 636).is_err());
        assert!(ldaps_url("dc01.example.com/path", 636).is_err());
    }

    #[tokio::test]
    async fn ldaps_uses_fixed_target_ip_with_configured_dns_sni_and_trusted_ca() {
        let server = LoopbackLdapsServer::spawn();
        let credential = loopback_credential(server.address.port(), Some(server.root_pem.clone()));
        let handle = connect_and_bind(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            &credential,
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        drop(handle);

        let (sni, request) = server.finish();
        assert_eq!(sni.as_deref(), Some("dc01.example.test"));
        assert!(request.starts_with(&[0x30]));
        assert!(
            request
                .windows(16)
                .any(|bytes| bytes == b"fixture-password")
        );
    }

    #[tokio::test]
    async fn ldaps_rejects_untrusted_certificate_and_sanitizes_transport_failure() {
        let server = LoopbackLdapsServer::spawn();
        let credential = loopback_credential(server.address.port(), None);
        let error = match connect_and_bind(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            &credential,
            &CancellationToken::new(),
        )
        .await
        {
            Ok(_) => panic!("untrusted LDAPS certificate was accepted"),
            Err(error) => error,
        };
        let debug_error = error.to_string();
        let failure = sanitized_failure("LDAPS connection or bind failed", error);

        let (sni, request) = server.finish();
        assert_eq!(sni.as_deref(), Some("dc01.example.test"));
        assert!(request.is_empty());
        assert_eq!(failure.message(), "LDAPS connection or bind failed");
        assert!(!failure.message().contains(&debug_error));
        assert!(!failure.message().contains("dc01.example.test"));
    }

    #[cfg(unix)]
    #[test]
    fn system_ccache_accepts_only_one_absolute_file_cache() {
        assert_eq!(
            system_ccache_path("FILE:/run/scanopy-krb5/ccache").unwrap(),
            PathBuf::from("/run/scanopy-krb5/ccache")
        );
        assert!(system_ccache_path("DIR:/run/scanopy-krb5").is_err());
        assert!(system_ccache_path("FILE:relative-cache").is_err());
        assert!(system_ccache_path("FILE:").is_err());
    }

    #[test]
    fn normalized_collection_is_valid_and_contains_no_raw_dns() {
        let now = Utc::now();
        let group_guid = uuid::Uuid::new_v4();
        let member_guid = uuid::Uuid::new_v4();
        let request = normalize_collection(
            AdInventory {
                domain: Some(AdDomain {
                    dns_name: "example.com".to_string(),
                    forest_dns_name: Some("example.com".to_string()),
                    functional_level: Some("7".to_string()),
                }),
                groups: vec![AdGroup {
                    distinguished_name: "CN=Readers,OU=Groups,DC=example,DC=com".to_string(),
                    object_guid: Some(group_guid),
                    name: Some("Readers".to_string()),
                    member_guids: vec![member_guid],
                }],
                ..Default::default()
            },
            AdCollectionIdentity {
                network_id: uuid::Uuid::new_v4(),
                credential_id: uuid::Uuid::new_v4(),
                target_host_id: uuid::Uuid::new_v4(),
                target_ip: "192.0.2.10".parse().unwrap(),
                discovery_id: uuid::Uuid::new_v4(),
                session_id: uuid::Uuid::new_v4(),
            },
            now,
            now,
        )
        .unwrap();

        request.validate().unwrap();
        let json = serde_json::to_string(&request).unwrap();
        assert!(!json.contains("CN=Readers"));
        assert!(!json.contains("CN=Alice"));
        assert!(!json.contains("OU=People"));
        assert!(json.contains(&member_guid.to_string()));
        assert_eq!(request.domains[0].entities.len(), 2);
        assert!(
            request.domains[0]
                .entities
                .iter()
                .all(|entity| uuid::Uuid::parse_str(&entity.external_id).is_ok())
        );
    }

    #[test]
    fn normalized_text_rejects_control_characters() {
        assert_eq!(
            bounded_text(Some("Windows Server"), 256).as_deref(),
            Some("Windows Server")
        );
        assert!(bounded_text(Some("Windows\nServer"), 256).is_none());
        assert!(bounded_text(Some("site\0name"), 256).is_none());
    }

    #[test]
    fn sanitized_failures_do_not_expose_local_paths() {
        let sentinel = "/sensitive/daemon/ccache-or-ca.pem";
        let failure = sanitized_failure(
            "credential material could not be resolved",
            anyhow!("failed to open {sentinel}"),
        );
        assert!(!failure.message().contains(sentinel));
        assert_eq!(
            failure.message(),
            "credential material could not be resolved"
        );
    }

    #[tokio::test]
    async fn controlled_operation_honors_cancellation_and_timeout() {
        let cancel = CancellationToken::new();
        cancel.cancel();
        assert!(
            controlled(
                &cancel,
                Duration::from_secs(1),
                std::future::pending::<()>()
            )
            .await
            .unwrap_err()
            .to_string()
            .contains("cancelled")
        );

        assert!(
            controlled(
                &CancellationToken::new(),
                Duration::from_millis(5),
                std::future::pending::<()>(),
            )
            .await
            .unwrap_err()
            .to_string()
            .contains("timed out")
        );
    }
}

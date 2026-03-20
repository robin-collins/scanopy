# Discovery Architecture: Generic Credential-Based Integrations

## 1. Current State

### Phase Orchestration (`unified.rs`)

`UnifiedDiscovery` (line 29) orchestrates three hardcoded phases with progress allocation via `PhaseAllocations` (line 39):

1. **Self-report phase** (lines 534–673) — First run only. Detects daemon's own interfaces, creates the daemon host entity with a `ScanopyDaemon` service.

2. **Docker phase** (lines 676–791) — Runs if `scan_local_docker_socket` is enabled or any `DockerProxy` credential exists in `credential_mappings` (checked by `should_run_docker_phase`, line 262). Resolves Docker proxy config via `resolve_docker_proxy()` (line 349), which only returns localhost credentials — it searches `ip_overrides` for localhost entries first, then falls back to AppConfig.

3. **Network scan phase** (lines 483–531) — Creates a `NetworkScanDiscovery` runner. SNMP credentials are extracted from `credential_mappings` via `extract_snmp_credential_mapping()` (line 280), which filters for `CredentialQueryPayload::Snmp` variants and resolves file paths.

### Credential Flow: Server → Daemon

`CredentialService::build_credential_mappings_for_discovery()` (`server/credentials/service.rs:342`) builds one `CredentialMapping<CredentialQueryPayload>` per `CredentialTypeDiscriminants` value:

- Fetches all hosts + interfaces on the network
- Network-level credentials become `default_credential`
- Host-level assignments become `IpOverride` entries (one per interface IP)
- `seed_ips` on credentials add additional `IpOverride` entries for bootstrap scenarios
- Each mapping carries `required_ports` from `CredentialType::required_ports()`

The mappings are sent to the daemon as part of `DaemonDiscoveryRequest`. On the daemon, `unified.rs` dispatches them to type-specific extraction methods.

### SNMP Integration (`network.rs`)

SNMP runs inline during `deep_scan_host()` (line 992). After TCP/UDP port scanning, if port 161 or 1161 is open (line 1172–1180), the function:

1. Tries SNMP credentials in specificity order (line 1199)
2. Queries system info, interface table, LLDP/CDP neighbors, ARP table, bridge FDB, device inventory (lines 1218–1318)
3. **Discovers remote subnets** from `ipAddrTable` (line 1485) — creates new `Subnet` entities and `Interface` entities for each
4. **Creates hosts from ARP entries** on those remote subnets (line 1586)
5. Enriches the host with SNMP data: `sys_descr`, `sys_name`, MAC address, chassis ID, manufacturer/model/serial (lines 1430–1480)

This inline execution is a **fundamental requirement**: SNMP data discovers new subnets and hosts that expand the scan scope within the same discovery session.

### Docker Integration (`docker.rs`)

`DockerScanDiscovery` (line 54) uses `OnceLock<Docker>` for the client. When run standalone, it connects via `new_local_docker_client()` (line 99). When run from unified, the client is set externally (unified.rs line 724).

Docker scanning discovers container hosts by listing containers, inspecting each for network info, and creating host + service + interface entities. It only works against the daemon's own Docker socket — `new_local_docker_client()` always connects to localhost.

### Credential Types (`server/credentials/impl/types/mod.rs`)

`CredentialType` (line 37) is a tagged enum with two variants:
- `SnmpV2c { community }` — `required_ports()` returns `[Snmp, SnmpAlt]` (161/udp, 1161/udp)
- `DockerProxy { port, path, ssl_cert, ssl_key, ssl_chain }` — `required_ports()` returns `[new_tcp(port)]`

Each variant maps to a `CredentialQueryPayload` variant via `to_query_payload()` (line 249).

### Service Matching (`server/services/impl/patterns.rs`, `definitions.rs`)

`ServiceDefinition` trait (definitions.rs line 20) requires `discovery_pattern() -> Pattern<'_>`. The `Pattern` enum (patterns.rs line 148) supports: `Port`, `Endpoint` (HTTP response matching), `Header`, `MacVendor`, `SubnetIsType`, `IsGateway`, `DockerContainer`, `Custom`, and composites (`AnyOf`, `AllOf`, `Not`).

During `deep_scan_host`, all registered `ServiceDefinition`s are evaluated against the host's open ports and endpoint responses (`discover_services` in base.rs line 560). The Pattern engine already handles service detection — credentials just aren't involved in that pipeline.

### `CredentialMapping<T>` (`server/credentials/impl/mapping.rs`)

Generic mapping with `default_credential: Option<T>` and `ip_overrides: Vec<IpOverride<T>>`. `get_credential_for_ip()` (line 64) resolves per-IP, falling back to default. Each mapping also carries `required_ports: Vec<PortType>`.

---

## 2. Proposed Architecture

### Integration Trait

```rust
/// Trait for credential-driven discovery integrations.
/// Each implementation handles one credential type's discovery logic.
///
/// Integrations return a single flexible result type (IntegrationResult)
/// and populate only the fields relevant to what they discovered.
/// There are no rigid categories — an integration can enrich the host,
/// add services, create new devices, or any combination.
#[async_trait]
trait DiscoveryIntegration: Send + Sync {
    /// Which credential type this integration handles
    fn credential_type(&self) -> CredentialTypeDiscriminants;

    /// When this integration runs in the discovery pipeline.
    /// PerHost: during deep_scan_host, after port scan + service matching.
    /// Controller: in a dedicated phase, querying a management endpoint.
    fn phase(&self) -> IntegrationPhase;

    /// Estimated seconds per host (PerHost) or per controller call.
    /// Used for progress estimation, not scheduling or timeouts.
    fn estimated_seconds(&self) -> u32;

    /// Maximum execution time before the caller cancels this integration.
    fn timeout(&self) -> Duration {
        Duration::from_secs(60)
    }

    /// Execute the integration. Called with appropriate context based on phase().
    /// Returns a flexible result — populate only what this integration discovers.
    async fn execute(
        &self,
        ctx: &IntegrationContext<'_>,
    ) -> Result<IntegrationResult, Error>;
}

/// When an integration runs — the only scheduling distinction that matters.
enum IntegrationPhase {
    /// Runs per-host during deep_scan_host, after port scan + service matching.
    /// Triggered when the associated service is detected AND credential is available.
    PerHost,
    /// Runs in a dedicated phase, querying a management endpoint.
    /// Not tied to individual host scanning.
    Controller,
}
```

**Why no data categories (HostEnrichment, EntityDiscovery, etc.):** Each integration returns a different mix of data — SNMP enriches host fields + adds interfaces + adds if_entries; Docker adds services with virtualization metadata; SSH enriches host fields + might add services; Ubiquiti creates new device hosts; vSphere creates VMs AND enriches the hypervisor. Forcing these into rigid categories doesn't reflect reality. Instead, integrations share a single `IntegrationResult` type and populate what they need.

### Integration Dispatch Flow

```
credential_mappings (from server)
    ↓
For each CredentialTypeDiscriminants:
    ↓
IntegrationRegistry::get(discriminant) → Box<dyn DiscoveryIntegration>
    ↓
Partition by phase():
    PerHost     → collected for post-scan dispatch in deep_scan_host
    Controller  → collected for controller phase

(SNMP is not in the registry — it remains a direct parameter
 to NetworkScanDiscovery, running inline during port scanning)
```

### Credential → ServiceDefinition Association

Currently, `CredentialType::required_ports()` duplicates what `ServiceDefinition::discovery_pattern()` already declares. The fix:

1. **Each `CredentialType` variant maps to a `ServiceDefinition` ID** — not just ports. Add a method:
   ```rust
   impl CredentialType {
       /// The ServiceDefinition this credential integrates with.
       /// When this service is detected on a host, the credential's
       /// integration runs automatically.
       fn associated_service(&self) -> Option<&'static str> {
           match self {
               Self::SnmpV2c { .. } => None, // ScanEnrichment, not service-triggered
               Self::DockerProxy { .. } => Some("Docker"),
           }
       }
   }
   ```

2. **Integration trigger during deep_scan_host:**
   ```
   port scan → service matching (existing Pattern engine)
       → for each matched service:
           → find credentials where associated_service() == service.id
           → if credential exists for this host (IP override or default):
               → run integration.discover_for_host()
   ```

3. **`required_ports()` becomes a convenience/validation method** — the authoritative port information lives in the ServiceDefinition's `discovery_pattern()`. `required_ports()` can remain for credential form validation ("this credential needs port X open") but is no longer used for dispatch.

### Discovery Phases (restructured `unified.rs`)

```
Phase 1: Daemon Host
├── Self-report (unchanged)
└── Localhost integrations:
    For each PerHost integration with credential targeting 127.0.0.1:
        Run integration.execute() with localhost context
    (Docker on localhost is just one case — any localhost credential runs here)
    Results merged into daemon host entity

Phase 2: Network Scan
├── ARP discovery (unchanged)
└── deep_scan_host per IP:
    ├── TCP/UDP port scanning (unchanged)
    ├── SNMP inline (still special-cased, not in integration registry)
    ├── Endpoint scanning (unchanged)
    ├── Service matching (unchanged)
    └── Post-scan integrations:
        For each PerHost integration with credential for this IP:
            Run integration.execute()
        IntegrationResult merged into host before create_host()

Phase 3: Controller
    For each Controller-phase integration:
        Run integration.execute()
    IntegrationResult.discovered_devices → create_host() for each
```

### Progress Model

#### Current Progress System

`PhaseAllocations` (unified.rs:39) assigns each phase a fixed percentage range. `DiscoverySession::set_progress_range(start, end)` (base.rs:120) maps a phase's internal 0–100% to its slice of overall progress. Within the network phase, progress is batch-based: `batches_completed / total_batches` (network.rs:58–61), with sub-phases weighted as ARP 0–30%, deep scan 30–95%, grace period 95–100%.

This works when port scanning dominates wall-clock time. With a light scan mode (not scanning all 65k ports) and post-scan integrations, API calls become a significant fraction of per-host time, and the batch-based model underestimates remaining work.

#### Cost-Based Estimation

Replace fixed percentage allocation with **estimated-seconds-based progress**. Each work unit declares its expected cost, and progress tracks `completed_seconds / total_estimated_seconds`.

**Each integration declares its cost** via `estimated_seconds()` on the trait (see Integration Trait above).

Example values: Docker ~5s/host (list + inspect containers), SSH ~3s/host (connect + run commands), Ubiquiti controller ~10s flat (one API call returning all devices), SNMP ~4s/host (already baked into base scan cost).

**Before scanning, compute total estimated work:**

```
total_estimated_seconds = 0

// Base scan cost per host (varies by scan mode, includes SNMP inline)
scan_seconds_per_host = match scan_mode {
    Full => 90,   // 65k port scan
    Light => 8,   // discovery ports only
}

// After ARP: know responsive host count
responsive_host_count = arp_results.len() + non_interfaced_responsive.len()
total_estimated_seconds += scan_seconds_per_host * responsive_host_count

// Integration costs from credential_mappings
for each credential_mapping:
    integration = IntegrationRegistry::get(mapping.credential_type)
    match integration.phase():
        PerHost:
            if mapping.default_credential.is_some():
                // Network-wide: applies to every responsive host
                total += integration.estimated_seconds() * responsive_host_count
            else:
                // IP overrides only: count matching responsive IPs
                matching = mapping.ip_overrides.filter(|o| responsive_ips.contains(o.ip))
                total += integration.estimated_seconds() * matching.len()
        Controller:
            total += integration.estimated_seconds()
```

**During scanning, accumulate completed work:**
- Port scan batch completes → add `batch_seconds` to `completed_estimated_seconds`
- Integration completes on a host → add `integration.estimated_seconds()` to completed
- Controller integration completes → add `integration.estimated_seconds()` to completed
- Progress = `completed / total` mapped to 0–100%

**Refinement after ARP:** When ARP completes and responsive host count is known, recalculate `total_estimated_seconds`. Hosts that didn't respond drop out of the denominator. This prevents progress from stalling at low percentages when most hosts on a large subnet are offline.

**Phase 1 (Daemon Host):** Fixed small allocation (0–5%). Self-report + localhost integrations are fast and predictable. Not worth cost-based estimation.

**Phase 2 (Network Scan):** Uses the cost-based model. Progress range is 5% to `100% - controller_allocation%`.

**Phase 3 (Controller):** Gets a proportional allocation based on `sum(estimated_seconds_per_controller) / total_estimated_seconds`. If no controller credentials exist, Phase 2 gets the full remaining range.

### Integration Context and Result Types

#### Integration Context

A single context type serves both PerHost and Controller integrations. PerHost-specific fields are `Option` — populated for per-host calls, `None` for controller calls.

```rust
struct IntegrationContext<'a> {
    /// Target IP — the scanned host (PerHost) or management endpoint (Controller)
    ip: IpAddr,
    /// The resolved credential (type-erased, integration downcasts to its variant)
    credential: &'a CredentialQueryPayload,
    /// Credential ID for auto-assignment tracking
    credential_id: Option<Uuid>,
    /// Network and daemon context
    network_id: Uuid,
    daemon_id: Uuid,
    /// Cancellation token — integration must check periodically
    cancel: &'a CancellationToken,
    /// Discovery type for entity source metadata
    discovery_type: &'a DiscoveryType,

    // --- PerHost-only fields (None for Controller calls) ---

    /// Open ports detected during scanning
    open_ports: Option<&'a [PortType]>,
    /// Services matched by the Pattern engine
    matched_services: Option<&'a [Service]>,
    /// Endpoint responses from HTTP probing
    endpoint_responses: Option<&'a [EndpointResponse]>,
}
```

The credential is passed as `&CredentialQueryPayload`. Each integration downcasts to its expected variant:
```rust
// Inside DockerIntegration::execute():
let CredentialQueryPayload::DockerProxy(cred) = ctx.credential else {
    return Err(anyhow!("Expected DockerProxy credential"));
};
```

This is type-safe at the dispatch level — `IntegrationRegistry` ensures the correct credential type reaches each integration — but avoids generic type parameters threading through the entire call chain.

#### Integration Result

A single flexible result type. Integrations populate only the fields they produce. No categories — Docker adds services, SSH enriches host fields, Ubiquiti creates devices, vSphere does both.

```rust
#[derive(Default)]
struct IntegrationResult {
    // --- Host enrichment (merged into scanned host before create_host) ---

    /// Host field overrides — only populated fields are merged
    sys_descr: Option<String>,
    sys_name: Option<String>,
    manufacturer: Option<String>,
    model: Option<String>,
    serial_number: Option<String>,
    management_url: Option<String>,

    /// Additional services to add to the host.
    /// Docker: container services with ServiceVirtualization::Docker
    /// SSH: discovered running services (e.g., systemd units)
    additional_services: Vec<Service>,

    /// Additional ports discovered by this integration
    additional_ports: Vec<Port>,

    /// Additional interfaces (e.g., SNMP remote interfaces on a router)
    additional_interfaces: Vec<Interface>,

    /// SNMP if_entries
    if_entries: Vec<IfEntry>,

    /// Credential assignment to record on the host
    credential_assignment: Option<CredentialAssignment>,

    // --- New device discovery (controller integrations) ---

    /// Devices discovered by querying a management API.
    /// Each becomes a new host entity via create_host().
    /// PerHost integrations leave this empty.
    discovered_devices: Vec<DiscoveredDevice>,
}

/// A device discovered by a controller integration (Ubiquiti AP, vSphere VM, etc.)
struct DiscoveredDevice {
    host: Host,
    interfaces: Vec<Interface>,
    ports: Vec<Port>,
    services: Vec<Service>,
}
```

**Why one result type:** Every integration returns the same struct, populating only what it needs:
- **Docker** → `additional_services` (container services with `ServiceVirtualization::Docker`)
- **SSH** → `sys_descr`, `manufacturer`, `additional_services` (systemd units)
- **Ubiquiti** → `discovered_devices` (APs, switches, gateways as new hosts)
- **vSphere** → `discovered_devices` (VMs) + host enrichment on the hypervisor (model, serial)

This avoids the problem where rigid categories force integrations into boxes that don't fit. A future integration that both enriches the host AND discovers sub-devices just fills in both sections.

**Merging (PerHost):** `deep_scan_host()` collects `IntegrationResult`s from all integrations that ran, then merges into the host before `create_host()`. First-write-wins for scalar fields (first integration to set `manufacturer` wins). Lists (services, interfaces, ports) are concatenated. This generalizes the current SNMP enrichment pattern (network.rs lines 1430–1480).

**Entity creation (Controller):** `run_controller_phase` collects `IntegrationResult`s. For each, it merges host enrichment into the controller host, then iterates `discovered_devices` and calls `create_host()` for each. The integration never calls `create_host()` directly — it returns data, the phase runner creates entities.

### Integration Registry

Static registry mapping credential types to integrations:

```rust
struct IntegrationRegistry;

impl IntegrationRegistry {
    /// Get the integration for a credential type, if one exists.
    /// Returns None for credential types without discovery integrations
    /// (e.g., future "API key" credentials that are used for monitoring, not discovery).
    fn get(discriminant: CredentialTypeDiscriminants) -> Option<Box<dyn DiscoveryIntegration>> {
        match discriminant {
            CredentialTypeDiscriminants::DockerProxy => Some(Box::new(DockerIntegration)),
            CredentialTypeDiscriminants::UnifiApi => Some(Box::new(UnifiIntegration)),
            CredentialTypeDiscriminants::Ssh => Some(Box::new(SshIntegration)),
            // SnmpV2c is NOT here — SNMP runs inline during scanning, not via the trait
            _ => None,
        }
    }
}
```

**Why static, not dynamic:** The set of integrations is known at compile time. Each integration is a zero-sized struct implementing the trait. No need for runtime registration, plugin loading, or configuration. Adding an integration means adding a struct + trait impl + match arm — the same pattern as `ServiceDefinition` registrations.

**Credential extraction from mappings:** When `unified.rs` prepares integrations for dispatch, it pairs each integration with its relevant `CredentialMapping` and partitions by phase:

```rust
// In run_unified_phases, before network scan:
let mut per_host_integrations: Vec<(Box<dyn DiscoveryIntegration>, CredentialMapping<CredentialQueryPayload>)> = vec![];
let mut controller_integrations: Vec<(Box<dyn DiscoveryIntegration>, CredentialMapping<CredentialQueryPayload>)> = vec![];

for mapping in &self.domain.credential_mappings {
    let discriminant = mapping.credential_type_discriminant();
    if let Some(integration) = IntegrationRegistry::get(discriminant) {
        match integration.phase() {
            IntegrationPhase::PerHost => {
                per_host_integrations.push((integration, mapping.clone()));
            }
            IntegrationPhase::Controller => {
                controller_integrations.push((integration, mapping.clone()));
            }
        }
    }
}
```

This requires `CredentialMapping<CredentialQueryPayload>` to expose its credential type discriminant. Currently the mapping is generic over `T` and doesn't know which variant it holds. Add:

```rust
impl CredentialMapping<CredentialQueryPayload> {
    fn credential_type_discriminant(&self) -> Option<CredentialQueryPayloadDiscriminants> {
        self.default_credential.as_ref().map(|c| c.discriminant())
            .or_else(|| self.ip_overrides.first().map(|o| o.credential.discriminant()))
    }
}
```

### Relationship to Existing Discovery Traits

The codebase has an existing trait system for discovery orchestration:

| Trait | Location | Purpose |
|-------|----------|---------|
| `RunsDiscovery` | base.rs:175 | Top-level entry: `discovery_type()` + `discover()` |
| `DiscoversNetworkedEntities` | base.rs:292 | Subnet creation, session lifecycle (`start_discovery`, `finish_discovery`) |
| `CreatesDiscoveredEntities` | base.rs:673 | `create_host()`, `create_subnet()` with DaemonPoll/ServerPoll buffering |
| `DiscoveryRunner<T>` | base.rs:74 | Generic runner struct, holds `DaemonDiscoveryService` + domain |

These are implemented for three domain types: `UnifiedDiscovery`, `NetworkScanDiscovery`, `DockerScanDiscovery`.

**The new `DiscoveryIntegration` trait does not replace these.** They operate at different levels:

- **Existing traits = orchestration layer.** They manage discovery sessions, create subnets, buffer entities for server communication, handle progress reporting and retry. This is infrastructure.
- **`DiscoveryIntegration` = plugin layer.** It defines how a specific credential type adds data to a host or queries a controller. It consumes the infrastructure (via context objects) but doesn't manage it.

**What stays unchanged:**
- `RunsDiscovery` — `DiscoveryRunner<UnifiedDiscovery>` remains the top-level entry point
- `DiscoversNetworkedEntities` — session lifecycle, subnet creation unchanged
- `CreatesDiscoveredEntities` — `create_host()` / `create_subnet()` with DaemonPoll/ServerPoll buffering unchanged
- `DiscoveryRunner<T>` — generic runner pattern unchanged
- `DiscoveryRunner<NetworkScanDiscovery>` — still owns ARP scanning, deep_scan_host, the scan pipeline. Post-scan integrations are called from within `deep_scan_host`

**What changes:**
- `DiscoveryRunner<DockerScanDiscovery>` — **absorbed into `DockerIntegration`**. Standalone Docker discovery (the separate `DiscoveryType::Docker` runner) goes away. Docker scanning becomes a `PerHost` integration that runs:
  - In Phase 1 against localhost (replacing `run_docker_phase`)
  - In Phase 2 against remote hosts (new capability)
  - The Docker scanning logic (`scan_and_process_containers`, `process_single_container`, etc.) moves from methods on `DiscoveryRunner<DockerScanDiscovery>` into `DockerIntegration::execute()`. The `DiscoversNetworkedEntities` impl for Docker (subnet creation for bridge networks) moves into the integration or is handled by the caller
- `run_unified_phases` in `DiscoveryRunner<UnifiedDiscovery>` — gains the controller phase, replaces hardcoded Docker/SNMP extraction with integration dispatch
- `extract_snmp_credential_mapping()` and `resolve_docker_proxy()` — replaced by generic credential dispatch through the registry

**What gets reused from Docker:**
- `scan_and_process_containers()` — core container scanning logic, reused inside `DockerIntegration`
- `process_single_container()` / `process_host_mode_container()` / `process_bridge_mode_container()` — container processing, reused
- `get_container_interfaces()`, `get_ports_from_container()` — container network mapping, reused
- `create_docker_daemon_service()` — creates the Docker daemon service entity, reused
- Docker subnet creation (`get_subnets_from_docker_networks`, `merge_host_and_docker_subnets`) — needs to happen before container scanning. For localhost Docker in Phase 1 this is handled by `DiscoveryRunner<UnifiedDiscovery>::discover_create_subnets()` (which already merges Docker subnets, unified.rs:130–151). For remote Docker in Phase 2, the remote host's subnets are already created by the network scan — Docker bridge subnets on a remote host are created by the integration as needed

**What's deleted:**
- `DiscoveryRunner<DockerScanDiscovery>` struct and its `RunsDiscovery` / `DiscoversNetworkedEntities` impls — the standalone Docker runner is no longer needed
- `DiscoveryType::Docker` variant — Docker no longer runs as its own discovery type
- `should_run_docker_phase()` — replaced by generic integration dispatch
- `run_docker_phase()` — replaced by `DockerIntegration::execute()` called from Phase 1 localhost integration loop

### Error Handling and Cancellation

**Cancellation:** Integrations receive `&CancellationToken` via their context. Long-running integrations (SSH command execution, large Docker container lists) must check `cancel.is_cancelled()` periodically — at minimum before each network call. The contract: if cancelled, return `Err(anyhow!("Discovery cancelled"))` promptly. The caller handles this the same as any other error.

**Per-integration timeouts:** Each integration declares `timeout()` on the trait (default 60s). The caller wraps execution:

```rust
let result = tokio::time::timeout(
    integration.timeout(),
    integration.execute(&ctx),
).await;
```

**Error isolation:** Integration failures are logged and skipped — they never block host creation or fail the phase. The host was already discovered via port scanning; the integration just failed to enrich it.

```rust
// In deep_scan_host, after service matching:
for (integration, mapping) in &per_host_integrations {
    if let Some(credential) = mapping.get_credential_for_ip(&ip) {
        let ctx = IntegrationContext { ip, credential, cancel, open_ports: Some(&open_ports), ... };
        match tokio::time::timeout(
            integration.timeout(),
            integration.execute(&ctx),
        ).await {
            Ok(Ok(result)) => {
                results.push(result);
                completed_estimated_seconds += integration.estimated_seconds();
            }
            Ok(Err(e)) => {
                tracing::warn!(
                    ip = %ip,
                    integration = %integration.credential_type(),
                    error = %e,
                    "Integration failed, continuing without enrichment"
                );
                completed_estimated_seconds += integration.estimated_seconds();
            }
            Err(_) => {
                tracing::warn!(
                    ip = %ip,
                    integration = %integration.credential_type(),
                    "Integration timed out after {:?}",
                    integration.timeout(),
                );
                completed_estimated_seconds += integration.estimated_seconds();
            }
        }
    }
}
```

Note: `completed_integration_seconds` is always incremented regardless of success/failure/timeout — the work unit is "done" either way for progress tracking purposes.

**Controller error isolation:** Same principle. A failed Ubiquiti API call doesn't prevent vSphere from running. Each controller integration runs independently. Failures are logged with the credential ID and target IP for debugging.

---

## 3. SNMP Exception

### Why SNMP Cannot Be Post-Scan

SNMP is a **scan-enrichment** integration, not a post-scan one. During `deep_scan_host()`, SNMP:

1. **Discovers remote subnets** via `ipAddrTable` (network.rs line 1485) — a router's SNMP data reveals subnets on its other interfaces. These subnets are created as entities and hosts from ARP entries on those subnets are created within the same `deep_scan_host` call.

2. **Provides MAC address enrichment** (network.rs line 1380) — when ARP scanning didn't provide a MAC, SNMP's `ipAddrTable` → `ifTable` mapping fills it in. This MAC is needed for the `Interface` entity created in the same call.

3. **Sets hostname from sysName** (network.rs line 1372) — used as fallback when DNS lookup fails, before the host entity is created.

4. **Enriches host fields** — `sys_descr`, `sys_object_id`, `sys_location`, `sys_contact`, `manufacturer`, `model`, `serial_number` are set on the host entity before creation (lines 1430–1480).

If SNMP ran post-scan, remote subnets and ARP-table hosts would not be discovered in that session. The host entity would need to be updated after creation (requiring an update API call), and MAC enrichment would be lost.

### How SNMP Fits the Model

SNMP is not in the `IntegrationRegistry` and does not implement `DiscoveryIntegration`. It's passed directly to `NetworkScanDiscovery` as today, running inline during `deep_scan_host()`.

This is an intentional asymmetry. The `DiscoveryIntegration` trait and registry exist for integrations that run *after* scanning. SNMP's inline nature means it's structurally different — it's part of the scan, not a consumer of scan results.

Future scan-enrichment integrations (unlikely but possible) would follow the same pattern: passed into `NetworkScanDiscovery` and called inline.

---

## 4. Remote Docker Design

### Current Limitation

`resolve_docker_proxy()` (unified.rs line 349) searches `credential_mappings` for `DockerProxy` credentials but only returns the first match, preferring localhost overrides. The Docker client is always created via `new_local_docker_client()`, which connects to localhost.

### Proposed Change

**Remove the localhost-only restriction.** During `deep_scan_host()`, after port scanning and service matching:

1. If a "Docker" service is matched on the host (Docker API port detected and matched by the Docker ServiceDefinition's `discovery_pattern()`)
2. AND a `DockerProxy` credential mapping exists for that host's IP (via `ip_override`) or as a network default
3. THEN run Docker scanning against that remote host

### Implementation Details

```rust
// In deep_scan_host, after service matching — generic integration dispatch:
for (integration, mapping) in &host_integrations {
    if let Some(credential) = mapping.get_credential_for_ip(&ip) {
        let ctx = HostIntegrationContext { ip, credential, cancel, ... };
        let result = integration.discover_for_host(&ctx).await?;
        // result.enrichment.additional_services contains container services
        // with ServiceVirtualization::Docker — merged into host before create_host()
    }
}
```

**Key changes to `resolve_docker_proxy()`:**
- Rename to `resolve_docker_credentials()` — returns a map of `IpAddr → DockerProxyQueryCredential`
- For each `IpOverride` with a `DockerProxy` credential, create an entry keyed by `override.ip`
- The `default_credential` applies to any host where the Docker service is detected but no specific override exists
- `new_local_docker_client()` becomes `new_docker_client(target_ip, cred)` — parameterized with target

**Progress impact:**
- Remote Docker scanning happens within `deep_scan_host()`, so it's covered by the network scan phase's progress allocation
- Each host with Docker adds a small constant time (container listing + inspection)
- No separate Docker phase needed for remote hosts — only localhost Docker still runs in Phase 1

**Localhost Docker unchanged:**
- Phase 1 (Daemon Host) still runs Docker against localhost using the existing flow
- This is because localhost Docker needs the daemon's own interface/subnet context, which is set up in Phase 1
- Remote Docker runs in Phase 2 (Network Scan) as a post-scan integration on each host

---

## 5. Future Integration Examples

### Ubiquiti (Controller phase — creates devices)

```rust
struct UnifiIntegration;

#[async_trait]
impl DiscoveryIntegration for UnifiIntegration {
    fn credential_type(&self) -> CredentialTypeDiscriminants {
        CredentialTypeDiscriminants::UnifiApi
    }
    fn phase(&self) -> IntegrationPhase { IntegrationPhase::Controller }
    fn estimated_seconds(&self) -> u32 { 10 }

    async fn execute(&self, ctx: &IntegrationContext<'_>) -> Result<IntegrationResult, Error> {
        let CredentialQueryPayload::UnifiApi(cred) = ctx.credential else { bail!("wrong type") };
        // 1. Connect to UniFi controller at ctx.ip
        // 2. GET /api/s/{site}/stat/device — returns all adopted devices
        // 3. For each device: build Host + interfaces from device.ip, device.mac
        // 4. Map device.type to services (UAP → WiFi AP, USW → Switch, etc.)
        Ok(IntegrationResult {
            discovered_devices: devices,
            ..Default::default()
        })
    }
}
```

**Credential type:** `UnifiApi { host, port, username, password, site }`.
**Trigger:** Controller-phase credentials run in Phase 3.

### Proxmox / vSphere (Controller phase — creates devices + enriches hypervisor)

```rust
struct ProxmoxIntegration;

#[async_trait]
impl DiscoveryIntegration for ProxmoxIntegration {
    fn credential_type(&self) -> CredentialTypeDiscriminants {
        CredentialTypeDiscriminants::ProxmoxApi
    }
    fn phase(&self) -> IntegrationPhase { IntegrationPhase::Controller }
    fn estimated_seconds(&self) -> u32 { 15 }

    async fn execute(&self, ctx: &IntegrationContext<'_>) -> Result<IntegrationResult, Error> {
        // 1. Connect to Proxmox API at https://{host}:8006/api2/json
        // 2. GET /nodes — list hypervisor nodes
        // 3. GET /nodes/{node}/qemu — list VMs per node
        // 4. GET /nodes/{node}/lxc — list containers
        // 5. Return VMs as discovered_devices + hypervisor model/serial as enrichment
        Ok(IntegrationResult {
            discovered_devices: vms,
            manufacturer: Some("Proxmox".into()),
            model: Some(node_info.model),
            ..Default::default()
        })
    }
}
```

vSphere follows the same pattern — creates VMs as `discovered_devices` and enriches the ESXi host.

### SSH (PerHost — enriches host fields + discovers services)

```rust
struct SshIntegration;

#[async_trait]
impl DiscoveryIntegration for SshIntegration {
    fn credential_type(&self) -> CredentialTypeDiscriminants {
        CredentialTypeDiscriminants::Ssh
    }
    fn phase(&self) -> IntegrationPhase { IntegrationPhase::PerHost }
    fn estimated_seconds(&self) -> u32 { 3 }

    async fn execute(&self, ctx: &IntegrationContext<'_>) -> Result<IntegrationResult, Error> {
        let CredentialQueryPayload::Ssh(cred) = ctx.credential else { bail!("wrong type") };
        // 1. SSH to ctx.ip using credential (key or password)
        // 2. Run: uname -a, hostnamectl, systemctl list-units --type=service
        // 3. Parse output → OS info, running services
        Ok(IntegrationResult {
            sys_descr: Some(uname_output),
            manufacturer: os_vendor,
            additional_services: discovered_services,
            ..Default::default()
        })
    }
}
```

**Credential type:** `Ssh { username, auth: KeyOrPassword }`.
**ServiceDefinition:** "SSH" — matched when port 22 is open.
**Trigger:** SSH service detected on host → SSH credential available → `execute()` runs post-scan.

### Aruba (Controller)

Same pattern as Ubiquiti — controller integration querying Aruba Central API or AOS-CX REST API for switch/AP topology data. Returns `discovered_devices` for managed APs and switches.

---

## 6. Implementation Plan

### Phase 1: Remote Docker (Minimal Change)

**Goal:** Enable Docker scanning on remote hosts without the full integration trait system.

**Changes:**
1. Modify `resolve_docker_proxy()` → `resolve_docker_credentials()` to return per-IP Docker credentials (not just localhost)
2. In `deep_scan_host()`, after service matching: if Docker service matched AND DockerProxy credential exists for that IP, run Docker scanning
3. Extract Docker scanning logic from `run_docker_phase()` into a reusable function parameterized by target IP and client
4. `new_local_docker_client()` → `new_docker_client(target: IpAddr, proxy_url, ssl_info)` — accepts target IP

**Dependencies:** None. Works with current `CredentialType` and `CredentialMapping` system.

**Risk:** Low. Remote Docker is additive — localhost Docker continues unchanged.

### Phase 2: Integration Trait + Registry

**Goal:** Introduce the `DiscoveryIntegration` trait and refactor Docker to use it.

**Changes:**
1. Define `DiscoveryIntegration` trait, `IntegrationPhase` enum, `IntegrationContext`, `IntegrationResult` in `daemon/discovery/integrations/mod.rs`
2. Create `IntegrationRegistry` — maps `CredentialTypeDiscriminants` → `Box<dyn DiscoveryIntegration>`
3. Implement `DockerIntegration` using the trait
4. Refactor `deep_scan_host()` to call PerHost integrations after service matching:
   - Get matched services → find associated credentials → dispatch to integration
5. Refactor `run_unified_phases()` to partition integrations by phase and dispatch

**Dependencies:** Phase 1 (remote Docker provides the parameterized scanning logic to wrap in the trait).

### Phase 3: Controller Phase

**Goal:** Add the controller phase to `unified.rs` for management-endpoint integrations.

**Changes:**
1. Extend `PhaseAllocations` with controller phase
2. Add `run_controller_phase()` to `DiscoveryRunner<UnifiedDiscovery>` — iterates Controller-phase integrations, calls `execute()`
3. Controller `IntegrationResult.discovered_devices` → `create_host()` for each
4. Controller `IntegrationResult` host enrichment fields → merge into controller host

**Dependencies:** Phase 2 (trait system).

### Phase 4: First Controller Integration (Ubiquiti)

**Goal:** Implement Ubiquiti as the first controller integration to validate the architecture.

**Changes:**
1. Add `UnifiApi` variant to `CredentialType`
2. Add "UniFi Controller" to `ServiceDefinitionRegistry`
3. Implement `UnifiIntegration` using the trait
4. Register in `IntegrationRegistry`

**Dependencies:** Phase 3 (controller phase).

### Phase 5: Additional Integrations

SSH, Proxmox, vSphere, Aruba — each follows the pattern:
1. Add `CredentialType` variant
2. Optionally add `ServiceDefinition` (if not already registered)
3. Implement `DiscoveryIntegration`
4. Register

**Dependencies:** Phase 2 for PerHost integrations, Phase 3 for Controller integrations.

### Dependency Graph

```
Phase 1 (Remote Docker)
    ↓
Phase 2 (Integration Trait)
    ↓
Phase 3 (Controller Phase) ← Phase 4 (Ubiquiti)
    ↓
Phase 5 (SSH, Proxmox, vSphere, Aruba)
```

---

## 7. Open Questions

1. **SNMP as trait or parameter?** The plan keeps SNMP as a direct parameter to `NetworkScanDiscovery` rather than going through the `DiscoveryIntegration` trait. Alternative: implement `DiscoveryIntegration` for SNMP with `ScanEnrichment` category, and have `deep_scan_host` call it at the right point. This would be more uniform but adds indirection for the only scan-enrichment integration. **Recommendation:** Keep SNMP as-is until a second scan-enrichment integration is needed.

2. **Controller credential targeting.** Controller integrations (Ubiquiti, vSphere) target a management endpoint, not individual hosts. How should the target IP be specified? Options:
   - `seed_ips` on the credential (current mechanism for bootstrap IPs)
   - A dedicated `controller_endpoint` field on the credential
   - Derived from the host the credential is assigned to (if the UniFi Controller host is discovered first, its interface IP is the target)

   **Recommendation:** Use host assignment — assign the credential to the controller host, and the integration uses that host's IP. `seed_ips` works as bootstrap before the controller host is discovered.

3. **Multiple credentials of the same type.** If two `DockerProxy` credentials target different hosts, `build_credential_mappings_for_discovery()` already handles this via `ip_overrides`. But what if two credentials of the same type target the same host? Currently only the first `ip_override` match wins. Is this sufficient?

   **Recommendation:** Yes — one credential per type per host is the intended model. The UI should prevent duplicate assignments.

4. **Light scan mode interaction.** The progress model assumes `scan_seconds_per_host` varies by scan mode. The light scan mode itself needs to be designed — what ports does it scan? Is it a `ScanSettings` option? Does the user choose per-discovery or is it a network-level default? This affects how `estimated_seconds_per_host` is calibrated.

5. **Integration cost calibration.** `estimated_seconds_per_host()` values are guesses until measured in production. Should we:
   - Ship with conservative estimates and tune later
   - Track actual durations per integration and adapt estimates over time (exponential moving average)
   - Both — start with static estimates, add adaptive tracking in a later phase

   **Recommendation:** Start with static estimates (Phase 2). Add adaptive tracking as a follow-up if progress accuracy is poor in practice. Over-engineering estimation is worse than slightly inaccurate progress bars.

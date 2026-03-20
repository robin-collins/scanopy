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

### Integration Traits

Two trait levels — a base trait for all integrations (including SNMP), and a registered trait for integrations dispatched through the registry:

```rust
/// Base trait shared by ALL integrations, including SNMP.
/// Provides metadata for progress estimation, credential type mapping,
/// and timeout configuration. Does NOT imply registry dispatch.
trait DiscoveryIntegration: Send + Sync {
    /// Which credential type this integration handles
    fn credential_type(&self) -> CredentialTypeDiscriminants;

    /// Estimated seconds per invocation (per-host or per-controller-call).
    /// Used for progress estimation, not scheduling or timeouts.
    fn estimated_seconds(&self) -> u32;

    /// Maximum execution time before the caller cancels this integration.
    fn timeout(&self) -> Duration {
        Duration::from_secs(60)
    }
}

/// Trait for integrations dispatched through the IntegrationRegistry.
/// These run either per-host (after port scanning) or as controller calls.
/// SNMP does NOT implement this — it runs inline during scanning.
#[async_trait]
trait RegisteredIntegration: DiscoveryIntegration {
    /// When this integration runs in the discovery pipeline.
    fn phase(&self) -> IntegrationPhase;

    /// Execute the integration. Returns a flexible result — populate only
    /// the fields relevant to what this integration discovers.
    async fn execute(
        &self,
        ctx: &IntegrationContext<'_>,
    ) -> Result<IntegrationResult, Error>;
}

/// When a registered integration runs.
enum IntegrationPhase {
    /// Runs per-host during deep_scan_host, after port scan + service matching.
    /// Triggered when the associated service is detected AND credential is available.
    PerHost,
    /// Runs in a dedicated phase, querying a management endpoint.
    /// Not tied to individual host scanning.
    Controller,
}
```

**Why two traits:** SNMP runs inline during port scanning (it discovers remote subnets that expand the scan — see section 3). It can't go through the `execute()` dispatch path. But it still shares metadata with other integrations: `credential_type()` for mapping, `estimated_seconds()` for progress, `timeout()` for safety. The base `DiscoveryIntegration` trait captures this shared contract. `RegisteredIntegration` adds the dispatch interface for integrations that run post-scan or as controllers.

```rust
// SNMP implements the base trait only:
struct SnmpIntegration;
impl DiscoveryIntegration for SnmpIntegration {
    fn credential_type(&self) -> CredentialTypeDiscriminants {
        CredentialTypeDiscriminants::SnmpV2c
    }
    fn estimated_seconds(&self) -> u32 { 4 } // system info + table walks
    fn timeout(&self) -> Duration { Duration::from_secs(30) }
}
// No RegisteredIntegration impl — SNMP is called inline, not via execute()

// Docker implements both:
struct DockerIntegration;
impl DiscoveryIntegration for DockerIntegration {
    fn credential_type(&self) -> CredentialTypeDiscriminants {
        CredentialTypeDiscriminants::DockerProxy
    }
    fn estimated_seconds(&self) -> u32 { 5 }
}
#[async_trait]
impl RegisteredIntegration for DockerIntegration {
    fn phase(&self) -> IntegrationPhase { IntegrationPhase::PerHost }
    async fn execute(&self, ctx: &IntegrationContext<'_>) -> Result<IntegrationResult, Error> {
        // ...
    }
}
```

**No data categories:** Each integration returns a different mix of data — SNMP enriches host fields + adds interfaces + adds if_entries; Docker adds services with virtualization metadata; SSH enriches host fields + might add services; Ubiquiti creates new device hosts; vSphere creates VMs AND enriches the hypervisor. Forcing these into rigid categories doesn't reflect reality. Instead, all registered integrations share a single `IntegrationResult` type and populate what they need.

### Integration Dispatch Flow

```
credential_mappings (from server)
    ↓
For each CredentialTypeDiscriminants:
    ↓
IntegrationRegistry::get(discriminant) → Option<Box<dyn RegisteredIntegration>>
    ↓
If registered:
    Partition by phase():
        PerHost     → collected for post-scan dispatch in deep_scan_host
        Controller  → collected for controller phase
If not registered (SNMP):
    Handled directly by NetworkScanDiscovery (inline during scanning)
    Still uses DiscoveryIntegration metadata for progress estimation
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
               → run integration.execute()
   ```

   The service match is the gate — if port 22 is open but no SSH service matched, the SSH integration doesn't run. This is correct: if the service match fails, the fix is to improve the service definition's `discovery_pattern()`, not to bypass matching. This keeps the existing Pattern engine as the single source of truth for service detection.

3. **Light scan mode must include credential-associated ports.** A light scan only checks a subset of ports for speed. But if a user has configured DockerProxy credentials for a host, the Docker API port (e.g., 2376) must be in the scan set even in light mode — otherwise the service won't match and the integration won't trigger. The light scan port list should be: `discovery_ports() ∪ ports from all credential-associated ServiceDefinitions`. This ensures credential-driven integrations always have a chance to trigger.

4. **`required_ports()` becomes a convenience/validation method** — the authoritative port information lives in the ServiceDefinition's `discovery_pattern()`. `required_ports()` can remain for credential form validation ("this credential needs port X open") but is no longer used for dispatch.

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

**Each integration declares its cost** via `estimated_seconds()` on the base `DiscoveryIntegration` trait — this includes SNMP even though it's not in the registry.

Example values: Docker ~5s/host, SSH ~3s/host, Ubiquiti controller ~10s flat, SNMP ~4s/host.

**Before scanning, compute total estimated work:**

```
total_estimated_seconds = 0

// Base scan cost per host (port scanning only, no integrations)
scan_seconds_per_host = match scan_mode {
    Full => 90,   // 65k port scan
    Light => 8,   // discovery ports only
}

// After ARP: know responsive host count
responsive_host_count = arp_results.len() + non_interfaced_responsive.len()
total_estimated_seconds += scan_seconds_per_host * responsive_host_count

// ALL integration costs — registered and unregistered (SNMP)
// Every credential_mapping has a DiscoveryIntegration with estimated_seconds()
for each credential_mapping:
    integration = lookup_integration(mapping.credential_type) // base trait, not registry
    if mapping.default_credential.is_some():
        // Network-wide: applies to every responsive host
        total += integration.estimated_seconds() * responsive_host_count
    else:
        // IP overrides only: count matching responsive IPs
        matching = mapping.ip_overrides.filter(|o| responsive_ips.contains(o.ip))
        total += integration.estimated_seconds() * matching.len()

// Controller integrations add a flat cost (not per-host)
for each controller_integration:
    total += integration.estimated_seconds()
```

Note: SNMP's `estimated_seconds()` participates in the total via the base trait, even though SNMP isn't in the `IntegrationRegistry`. The `lookup_integration()` function resolves ALL credential types to their `DiscoveryIntegration` impl for metadata — it's separate from registry dispatch.

**During scanning, accumulate completed work:**
- Port scan batch completes → add `batch_seconds` to `completed_estimated_seconds`
- Integration completes on a host → add `integration.estimated_seconds()` to completed
- Controller integration completes → add `integration.estimated_seconds()` to completed
- Progress = `completed / total` mapped to 0–100%

**Refinement after ARP:** When ARP completes and responsive host count is known, recalculate `total_estimated_seconds`. Hosts that didn't respond drop out of the denominator. This prevents progress from stalling at low percentages when most hosts on a large subnet are offline.

**Phase 1 (Daemon Host):** Fixed small allocation (0–5%). Self-report + localhost integrations are fast and predictable. Not worth cost-based estimation.

**Phase 2 (Network Scan):** Uses the cost-based model. Progress range is 5% to `100% - controller_allocation%`.

**Phase 3 (Controller):** Gets a proportional allocation based on `sum(estimated_seconds_per_controller) / total_estimated_seconds`. If no controller credentials exist, Phase 2 gets the full remaining range.

### Integration Context, Operations, and Result Types

#### Integration Context

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
    /// Operations handle — for integrations that need to create entities or report progress
    ops: &'a dyn IntegrationOps,

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

#### Integration Operations

Some integrations (Docker, controller integrations) need to perform side effects during execution — create subnets, create host entities, report progress. Rather than forcing all output through the return value, the context provides an `IntegrationOps` handle:

```rust
#[async_trait]
trait IntegrationOps: Send + Sync {
    /// Create a subnet (e.g., Docker bridge network).
    async fn create_subnet(&self, subnet: &Subnet) -> Result<Subnet, Error>;

    /// Create a discovered host entity with its children.
    /// Used by controller integrations to create devices incrementally
    /// (with progress reporting between each).
    async fn create_host(
        &self,
        host: Host,
        interfaces: Vec<Interface>,
        ports: Vec<Port>,
        services: Vec<Service>,
    ) -> Result<(), Error>;

    /// Report integration-level progress (0–100 within this integration's allocation).
    async fn report_progress(&self, percent: u8) -> Result<(), Error>;
}
```

Simple integrations (SSH) ignore `ops` entirely — they just return an `IntegrationResult`. Complex integrations use `ops` for what they need:

- **Docker** → `ops.create_subnet()` for bridge networks, then returns `additional_services` in the result
- **Ubiquiti** → `ops.create_host()` per device + `ops.report_progress()` between devices
- **SSH** → ignores ops, returns enrichment fields in result

The `IntegrationOps` implementation is provided by the caller (`deep_scan_host` or `run_controller_phase`) and delegates to the existing `CreatesDiscoveredEntities` infrastructure. This keeps entity creation, buffering, and retry logic centralized while giving integrations the ability to perform operations when needed.

#### Integration Result

Returned from `execute()`. Contains **enrichment data to merge into the scanned host**. Kept slim — complex operations go through `IntegrationOps`, not the return value.

```rust
/// Output from an integration. Each field is a discrete output type.
/// Integrations populate only what they produce.
#[derive(Default)]
struct IntegrationResult {
    /// Enrichment data to merge into the host entity before create_host().
    /// Each variant is independently optional.
    outputs: Vec<IntegrationOutput>,
}

enum IntegrationOutput {
    /// Host field enrichment — merged before create_host()
    HostField(HostFieldEnrichment),
    /// Service to add to the host (Docker container, SSH-discovered service, etc.)
    Service(Service),
    /// Port discovered by this integration
    Port(Port),
    /// Interface discovered (e.g., SNMP remote interface on a router)
    Interface(Interface),
    /// SNMP if_entry
    IfEntry(IfEntry),
    /// Credential to auto-assign to the host
    CredentialAssignment(CredentialAssignment),
}

/// Individual host field enrichments. Using an enum avoids a struct
/// with 20 Option<String> fields that grows with every integration.
enum HostFieldEnrichment {
    SysDescr(String),
    SysName(String),
    Manufacturer(String),
    Model(String),
    SerialNumber(String),
    ManagementUrl(String),
    // Future integrations add variants here without touching existing code
}
```

**Why composable outputs instead of a god struct:** A flat struct with `Option<String>` for every possible host field and `Vec<T>` for every entity type becomes unwieldy as integrations are added — every new output shape means a new field, most of which are empty for any given integration. The enum approach is composable: each integration emits only the variants it produces, and the merge logic in `deep_scan_host()` handles them uniformly. Adding a new output type means adding an enum variant and a merge handler, not modifying a struct that every integration touches.

**Merging (PerHost):** `deep_scan_host()` collects `IntegrationResult`s from all integrations that ran, iterates `outputs`, and applies each:
- `HostField` → set on host (first-write-wins for same field)
- `Service` → append to services list
- `Port` / `Interface` / `IfEntry` → append to respective lists
- `CredentialAssignment` → record on host

This generalizes the current SNMP enrichment pattern (network.rs lines 1430–1480).

**Entity creation (Controller):** Controller integrations create devices via `ops.create_host()` during execution, with `ops.report_progress()` between each. The return value contains enrichment for the controller host itself (if any). This avoids buffering hundreds of devices in memory just to return them.

### Concurrency Model

**Per-host integrations run sequentially on each host.** If a host has Docker (5s) + SSH (3s), they run one after the other. This avoids overwhelming the host with concurrent API calls / SSH sessions / Docker queries. The sequential overhead is acceptable because `deep_scan_host()` already runs concurrently across hosts (via the existing `buffer_unordered` stream in the network scan pipeline). With 15 concurrent host scans and 8s of integration time per host, throughput is limited by host concurrency, not integration serialization.

**Controller integrations run sequentially in Phase 3.** Each controller integration makes API calls to a different endpoint, so they could theoretically run in parallel. However, controller calls are fast relative to the network scan, and sequential execution simplifies error handling and progress reporting. Can be parallelized later if it becomes a bottleneck.

### Integration Registry

Two lookup mechanisms — one for dispatch (registered integrations only), one for metadata (all integrations including SNMP):

```rust
struct IntegrationRegistry;

impl IntegrationRegistry {
    /// Get a registered integration for dispatch via execute().
    /// Returns None for SNMP (inline) and credential types without integrations.
    fn get_registered(discriminant: CredentialTypeDiscriminants)
        -> Option<Box<dyn RegisteredIntegration>>
    {
        match discriminant {
            CredentialTypeDiscriminants::DockerProxy => Some(Box::new(DockerIntegration)),
            CredentialTypeDiscriminants::UnifiApi => Some(Box::new(UnifiIntegration)),
            CredentialTypeDiscriminants::Ssh => Some(Box::new(SshIntegration)),
            _ => None,
        }
    }

    /// Get base integration metadata for ANY credential type.
    /// Used for progress estimation — includes SNMP.
    fn get_metadata(discriminant: CredentialTypeDiscriminants)
        -> Option<Box<dyn DiscoveryIntegration>>
    {
        match discriminant {
            CredentialTypeDiscriminants::SnmpV2c => Some(Box::new(SnmpIntegration)),
            CredentialTypeDiscriminants::DockerProxy => Some(Box::new(DockerIntegration)),
            CredentialTypeDiscriminants::UnifiApi => Some(Box::new(UnifiIntegration)),
            CredentialTypeDiscriminants::Ssh => Some(Box::new(SshIntegration)),
            _ => None,
        }
    }
}
```

**Why static, not dynamic:** The set of integrations is known at compile time. Each integration is a zero-sized struct implementing the trait. No need for runtime registration, plugin loading, or configuration. Adding an integration means adding a struct + trait impl + match arm — the same pattern as `ServiceDefinition` registrations.

**Credential extraction from mappings:** When `unified.rs` prepares integrations for dispatch, it pairs each registered integration with its relevant `CredentialMapping` and partitions by phase:

```rust
// In run_unified_phases, before network scan:
let mut per_host_integrations: Vec<(Box<dyn RegisteredIntegration>, CredentialMapping<CredentialQueryPayload>)> = vec![];
let mut controller_integrations: Vec<(Box<dyn RegisteredIntegration>, CredentialMapping<CredentialQueryPayload>)> = vec![];

for mapping in &self.domain.credential_mappings {
    let discriminant = mapping.credential_type_discriminant();
    if let Some(integration) = IntegrationRegistry::get_registered(discriminant) {
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
- **`DiscoveryIntegration` = metadata layer.** Shared by all integrations (including SNMP). Declares credential type, estimated cost, timeout.
- **`RegisteredIntegration` = plugin layer.** Extends `DiscoveryIntegration` with `phase()` and `execute()`. Defines how a specific credential type adds data to a host or queries a controller. Consumed by the orchestration layer via context objects.

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

### Entity Creation Decoupling

Entity creation (`create_host()`, `create_subnet()`) is currently coupled to discovery sessions. This coupling must be broken for two reasons: (1) `IntegrationOps` needs a clean delegation target, and (2) passive integrations (section 6) need entity creation without any session context.

**Current coupling:**

- `CreatesDiscoveredEntities` (base.rs:673) requires `AsRef<DaemonDiscoveryService> + RunsDiscovery` — only `DiscoveryRunner<T>` satisfies this
- `create_host()` / `create_subnet()` use `DaemonDiscoveryService.entity_buffer` and `DaemonDiscoveryService.api_client`
- Entity buffer is cleared at session end (`clear_all()` in `finish_discovery`) — passive entities arriving between sessions would be lost
- Progress reporting is mixed into the creation path (e.g., buffer flush triggers progress updates)

**Proposed decoupling:**

Extract a standalone `EntityCreator` that depends only on the primitives needed for entity creation, not on session infrastructure:

```rust
/// Standalone entity creation — no session, no progress, no phase context.
/// Used by IntegrationOps (session-aware) and passive integrations (sessionless).
struct EntityCreator {
    api_client: ApiClient,
    entity_buffer: EntityBuffer,
    config_store: ConfigStore,  // DaemonPoll vs ServerPoll mode
    network_id: Uuid,
    daemon_id: Uuid,
}

impl EntityCreator {
    async fn create_host(
        &self,
        host: Host,
        interfaces: Vec<Interface>,
        ports: Vec<Port>,
        services: Vec<Service>,
    ) -> Result<(), Error>;

    async fn create_subnet(&self, subnet: &Subnet) -> Result<Subnet, Error>;

    /// Flush buffered entities to the server (for DaemonPoll mode).
    async fn flush(&self) -> Result<(), Error>;
}
```

**How the layers compose:**

- **`EntityCreator`** — owns entity creation logic. No session awareness. Handles DaemonPoll (buffer + POST) vs ServerPoll (buffer + wait) based on `config_store`.
- **`CreatesDiscoveredEntities`** — becomes a thin wrapper that adds session-aware behavior on top of `EntityCreator`: progress reporting after creation, session metadata stamping, buffer clearing on session end.
- **`IntegrationOps` impl** — delegates `create_host()` / `create_subnet()` to `EntityCreator` (via `CreatesDiscoveredEntities` during active sessions). Adds progress tracking.
- **Passive integrations** — use `EntityCreator` directly. No session, no progress, just entity creation and flush.

**Entity buffer lifecycle changes:**

Currently the entity buffer has a single lifecycle: cleared on session start, flushed during scanning, cleared on session end. With passive integrations, entities can arrive at any time. The buffer needs a dual lifecycle:

- **Session entities** — same as today. Buffer cleared on session start/end. Flushed in batches during scanning.
- **Passive entities** — buffered independently, flushed on a timer or threshold (e.g., every 30s or when buffer reaches N entities). Not tied to session lifecycle. If a session is active, passive entities can share the flush cycle but are not cleared when the session ends.

This could be implemented as two buffer partitions within the same `EntityBuffer`, or as separate buffer instances. The simpler approach is a separate `EntityBuffer` instance owned by the passive integration listener, with its own flush timer.

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
// In deep_scan_host, after service matching — sequential per host:
for (integration, mapping) in &per_host_integrations {
    if let Some(credential) = mapping.get_credential_for_ip(&ip) {
        let ops = HostIntegrationOpsImpl { /* delegates to CreatesDiscoveredEntities */ };
        let ctx = IntegrationContext { ip, credential, cancel, ops: &ops, ... };
        match tokio::time::timeout(
            integration.timeout(),
            integration.execute(&ctx),
        ).await {
            Ok(Ok(result)) => {
                results.push(result);
            }
            Ok(Err(e)) => {
                tracing::warn!(
                    ip = %ip,
                    integration = %integration.credential_type(),
                    error = %e,
                    "Integration failed, continuing without enrichment"
                );
            }
            Err(_) => {
                tracing::warn!(
                    ip = %ip,
                    integration = %integration.credential_type(),
                    "Integration timed out after {:?}",
                    integration.timeout(),
                );
            }
        }
        // Always increment — the work unit is "done" regardless of outcome
        completed_estimated_seconds += integration.estimated_seconds();
    }
}
```

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

SNMP implements the base `DiscoveryIntegration` trait but NOT `RegisteredIntegration`. It participates in the integration system for metadata:

- **`credential_type()`** → `CredentialTypeDiscriminants::SnmpV2c` — same mapping as any integration
- **`estimated_seconds()`** → ~4s — included in progress estimation via `get_metadata()`
- **`timeout()`** → 30s — used by `deep_scan_host` to cap SNMP per-host time

But it's not dispatched via `execute()`. SNMP credentials are still extracted by `extract_snmp_credential_mapping()` and passed as a parameter to `NetworkScanDiscovery::new()`. Execution remains inline in `deep_scan_host()` where SNMP data feeds back into the scan.

This is a deliberate split: SNMP is *defined* like other integrations but *executed* differently because of its scan-enrichment nature (see "Why SNMP Cannot Be Post-Scan" above). Future scan-enrichment integrations would follow the same pattern: implement `DiscoveryIntegration` for metadata, skip `RegisteredIntegration`, run inline.

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
for (integration, mapping) in &per_host_integrations {
    if let Some(credential) = mapping.get_credential_for_ip(&ip) {
        let ops = HostIntegrationOpsImpl { /* delegates to CreatesDiscoveredEntities */ };
        let ctx = IntegrationContext { ip, credential, ops: &ops, cancel, ... };
        let result = integration.execute(&ctx).await?;
        // result.outputs contains IntegrationOutput::Service entries with
        // ServiceVirtualization::Docker — merged into host before create_host()
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
        let devices = unifi_client.list_devices().await?;
        // 3. For each device: create host via ops (with progress reporting)
        for (i, device) in devices.iter().enumerate() {
            let host = build_host_from_unifi_device(device, ctx);
            ctx.ops.create_host(host, interfaces, ports, services).await?;
            ctx.ops.report_progress((i * 100 / devices.len()) as u8).await?;
        }
        // 4. Return enrichment for the controller host itself (if any)
        Ok(IntegrationResult::default())
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
        // 3. GET /nodes/{node}/qemu + /lxc — list VMs and containers
        // 4. Create each VM/container as a host via ops
        for vm in &vms {
            ctx.ops.create_host(build_proxmox_vm_host(vm), ...).await?;
        }
        // 5. Return hypervisor enrichment
        Ok(IntegrationResult {
            outputs: vec![
                IntegrationOutput::HostField(HostFieldEnrichment::Manufacturer("Proxmox".into())),
                IntegrationOutput::HostField(HostFieldEnrichment::Model(node_info.model)),
            ],
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
        let mut outputs = vec![
            IntegrationOutput::HostField(HostFieldEnrichment::SysDescr(uname_output)),
        ];
        if let Some(vendor) = os_vendor {
            outputs.push(IntegrationOutput::HostField(HostFieldEnrichment::Manufacturer(vendor)));
        }
        outputs.extend(discovered_services.into_iter().map(IntegrationOutput::Service));
        Ok(IntegrationResult { outputs })
    }
}
```

**Credential type:** `Ssh { username, auth: KeyOrPassword }`.
**ServiceDefinition:** "SSH" — matched when port 22 is open.
**Trigger:** SSH service detected on host → SSH credential available → `execute()` runs post-scan.

### Aruba (Controller)

Same pattern as Ubiquiti — controller integration querying Aruba Central API or AOS-CX REST API for switch/AP topology data. Returns `discovered_devices` for managed APs and switches.

---

## 6. Passive Integrations

### What They Are

Not all integrations are pull-based. Some integrations receive data **pushed from external systems** at arbitrary times, outside any discovery session. Examples:

- **DHCP lease notifications** — a DHCP server notifies the daemon when a new lease is granted. The daemon creates or updates a host entity with the leased IP and MAC.
- **SNMP traps** — network devices send traps (link up/down, threshold exceeded, device boot) to the daemon. The daemon creates or enriches host entities based on trap source.
- **Syslog messages** — devices forward syslog to the daemon. Parsed messages can reveal new hosts, services, or state changes.

### How They Differ from Active Integrations

| Aspect | Active (Pull) | Passive (Push) |
|--------|---------------|----------------|
| Trigger | Discovery session starts | External event arrives |
| Timing | During scheduled/manual scan | Any time, unpredictable |
| Session context | Has `DiscoverySession`, progress, phases | No session |
| Data flow | Daemon queries target | Target pushes to daemon |
| Scope | Known subnets/hosts | Any source IP |

Passive integrations don't need:
- **`IntegrationContext`** — no scan data, no open ports, no matched services, no cancellation token
- **`IntegrationPhase`** — not part of the scan pipeline
- **`IntegrationResult` merging** — no host being built up by `deep_scan_host()` to merge into
- **Service matching trigger** — events arrive without port scanning first
- **Progress tracking** — no session to report progress to

Passive integrations **do** need:
- **Entity creation** — create hosts, enrich existing hosts, add interfaces/services. This is the same `EntityCreator` infrastructure used by active integrations (see "Entity Creation Decoupling" in section 2).
- **Credential association** — a DHCP integration might need credentials to query the DHCP server for additional lease details
- **`IntegrationOutput`** — the same enum works for describing what was discovered (host fields, interfaces, services). The outputs just aren't merged during `deep_scan_host()` — they're applied directly via `EntityCreator`.
- **Network/daemon context** — `network_id` and `daemon_id` for entity creation

### Architecture Sketch

Passive integrations don't implement `DiscoveryIntegration` or `RegisteredIntegration` — those traits assume scan-time metadata (estimated_seconds, phase, credential_type for mapping). Instead, passive integrations are long-lived listeners:

```rust
/// A passive integration that listens for external events.
#[async_trait]
trait PassiveIntegration: Send + Sync {
    /// Human-readable name for logging
    fn name(&self) -> &'static str;

    /// Start listening. Runs until the cancellation token is triggered.
    /// Uses entity_creator to create/enrich entities as events arrive.
    async fn listen(
        &self,
        entity_creator: &EntityCreator,
        cancel: &CancellationToken,
    ) -> Result<(), Error>;
}
```

Each passive integration runs as a long-lived task (spawned by the daemon on startup, cancelled on shutdown). When an event arrives, it uses `EntityCreator` directly to create or enrich entities — no session, no progress, no phase.

### Design Implications

The key prerequisite is **entity creation decoupled from discovery sessions** (section 2, "Entity Creation Decoupling"). Without `EntityCreator` as a standalone component, passive integrations would need to fake a discovery session just to call `create_host()`.

This decoupling is planned for Phase 2 of the implementation plan (section 7), since `IntegrationOps` already needs a clean delegation target. Once `EntityCreator` exists, passive integrations can use it with no additional infrastructure.

---

## 7. Implementation Plan

### Phase 1: Remote Docker (Minimal Change)

**Goal:** Enable Docker scanning on remote hosts without the full integration trait system.

**Changes:**
1. Modify `resolve_docker_proxy()` → `resolve_docker_credentials()` to return per-IP Docker credentials (not just localhost)
2. In `deep_scan_host()`, after service matching: if Docker service matched AND DockerProxy credential exists for that IP, run Docker scanning
3. Extract Docker scanning logic from `run_docker_phase()` into a reusable function parameterized by target IP and client
4. `new_local_docker_client()` → `new_docker_client(target: IpAddr, proxy_url, ssl_info)` — accepts target IP

**Dependencies:** None. Works with current `CredentialType` and `CredentialMapping` system.

**Risk:** Low. Remote Docker is additive — localhost Docker continues unchanged.

### Phase 2: Integration Trait + Registry + Entity Creation Decoupling

**Goal:** Introduce the `DiscoveryIntegration` trait, decouple entity creation from sessions, and refactor Docker to use it.

**Changes:**
1. Define `DiscoveryIntegration` (base) and `RegisteredIntegration` traits, `IntegrationPhase` enum, `IntegrationContext`, `IntegrationResult` in `daemon/discovery/integrations/mod.rs`
2. Create `IntegrationRegistry` with `get_registered()` and `get_metadata()` lookups
3. Implement `DiscoveryIntegration` for `SnmpIntegration` (metadata only — no `RegisteredIntegration`)
4. Implement `DockerIntegration` using both traits
5. **Extract `EntityCreator`** from `CreatesDiscoveredEntities` — standalone struct with `create_host()`, `create_subnet()`, `flush()` that depends only on `api_client` + `entity_buffer` + `config_store`. `CreatesDiscoveredEntities` becomes a thin wrapper adding session-aware behavior. `IntegrationOps` delegates to `EntityCreator`.
6. Refactor `deep_scan_host()` to call PerHost integrations after service matching:
   - Get matched services → find associated credentials → dispatch to integration
7. Refactor `run_unified_phases()` to partition integrations by phase and dispatch

**Dependencies:** Phase 1 (remote Docker provides the parameterized scanning logic to wrap in the trait).

**Note:** Entity creation decoupling is done here because `IntegrationOps` needs a clean delegation target. This also unblocks passive integrations (section 6) as a future addition — they can use `EntityCreator` directly without faking a session.

### Phase 3: Controller Phase

**Goal:** Add the controller phase to `unified.rs` for management-endpoint integrations.

**Changes:**
1. Extend `PhaseAllocations` with controller phase
2. Add `run_controller_phase()` to `DiscoveryRunner<UnifiedDiscovery>` — iterates Controller-phase integrations, calls `execute()`
3. Provide `IntegrationOps` implementation that delegates `create_host()` / `create_subnet()` to existing `CreatesDiscoveredEntities`
4. Controller integrations use `ops.create_host()` per device and `ops.report_progress()` between devices

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
Phase 2 (Integration Trait + Entity Creation Decoupling)
    ↓                          ↘
Phase 3 (Controller Phase)    Future: Passive Integrations (use EntityCreator directly)
    ↓
Phase 4 (Ubiquiti)
    ↓
Phase 5 (SSH, Proxmox, vSphere, Aruba)
```

### Testing Strategy

Integrations must be testable in isolation without real Docker/SSH/SNMP/API endpoints.

**Unit testing integrations:**
- Each integration is a zero-sized struct. Test `execute()` with a mock `IntegrationContext` containing fake credentials, ports, services.
- `IntegrationOps` is a trait — provide a `MockIntegrationOps` that records calls to `create_host()`, `create_subnet()`, `report_progress()` without hitting the server.
- Assert on the returned `IntegrationResult` outputs and the ops calls made.

```rust
#[tokio::test]
async fn docker_integration_returns_container_services() {
    let mock_ops = MockIntegrationOps::new();
    let ctx = IntegrationContext {
        ip: "192.168.1.100".parse().unwrap(),
        credential: &docker_proxy_credential(),
        ops: &mock_ops,
        // ...
    };
    let result = DockerIntegration.execute(&ctx).await.unwrap();
    assert!(result.outputs.iter().any(|o| matches!(o, IntegrationOutput::Service(_))));
    assert_eq!(mock_ops.subnets_created(), 1); // Docker bridge
}
```

**Integration dispatch testing:**
- Test the full dispatch path: credential mapping → registry lookup → integration execution → result merging → host creation.
- Use `MockIntegrationOps` + a fake `IntegrationRegistry` (or the real one with mock credentials).
- Verify: correct integrations are called for each host based on service matches, results are merged correctly, progress is reported.

**Regression testing for SNMP inline:**
- SNMP doesn't go through `execute()`, so test it separately. Existing SNMP unit tests cover this.
- Add a test verifying that SNMP's `estimated_seconds()` is included in progress calculation via `get_metadata()`.

**Phase 2 deliverable:** The integration trait + `MockIntegrationOps` + at least one tested integration (Docker) before moving to Phase 3.

### Controller Phase Ordering

The doc places controllers in Phase 3 (after network scan). This ordering has trade-offs:

**Controllers after network scan (current proposal):**
- Pro: Network scan may discover the controller host first, providing its IP for the controller integration
- Pro: Avoids duplicate host creation — network scan creates hosts, controller enriches them
- Con: Controller-discovered devices aren't deep-scanned (no per-host integrations run on them)
- Con: Devices only reachable via the controller API (not on scanned subnets) won't be found until Phase 3

**Controllers before network scan (alternative):**
- Pro: Controller data could inform the network scan (e.g., skip scanning Ubiquiti APs that the controller already reported)
- Con: Controller host IP might not be known yet (no interfaces discovered)
- Con: Network scan would need deduplication against controller-created hosts

**Controllers in parallel with network scan (alternative):**
- Pro: No wasted wall-clock time waiting for one to finish before starting the other
- Con: Race conditions on host creation (both phases try to create the same host)
- Con: More complex progress model

**Decision: Phase 3 (after network scan) is correct for now.** The main risk — duplicate hosts from controller + network scan discovering the same device — is handled by the server's existing host deduplication on IP/MAC. Controller-discovered devices that aren't on scanned subnets are additive. The only real loss is that controller-discovered devices don't get deep-scanned, which is acceptable for the initial implementation. A future optimization could feed controller-discovered IPs back into the scan as additional targets.

---

## 8. Open Questions

1. **SNMP timeout enforcement.** SNMP implements `DiscoveryIntegration::timeout()` (30s) but currently `deep_scan_host` doesn't enforce a per-host SNMP timeout — individual SNMP queries have their own timeouts but the total per-host SNMP time is unbounded. Should we wrap the entire SNMP block in `tokio::time::timeout(snmp_integration.timeout(), ...)` for consistency with how registered integrations are called?

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

6. **Passive integration event model.** How does the daemon receive push data from external systems? Options:
   - **Listener threads** — daemon opens UDP/TCP sockets (e.g., UDP 162 for SNMP traps, UDP 514 for syslog). Each passive integration spawns a listener task on startup.
   - **Callback registration** — daemon exposes an HTTP endpoint that external systems POST to (e.g., DHCP server webhook). Passive integrations register routes.
   - **File/pipe watching** — daemon watches a file or named pipe (e.g., `dhcpd.leases` file, or a pipe from `dhcpd`).
   - **Combination** — different passive integrations use different mechanisms depending on the protocol.

   This doesn't need to be decided now (passive integrations are future work), but the `PassiveIntegration::listen()` trait should be flexible enough to accommodate all of these. The current sketch (long-lived async task with a cancellation token) supports all options.

7. **Controller-discovered hosts and deep scanning.** Controller integrations run in Phase 3 (after network scan), so devices they discover don't go through `deep_scan_host()`. If a controller surfaces hosts that didn't respond to ARP but are still reachable (e.g., on a different VLAN, or with ICMP disabled), they miss out on port scanning, service matching, and per-host integrations. Options:
   - **Accept the limitation** — controller-discovered devices are additive metadata. They'll get deep-scanned in the next discovery session if they're on a scanned subnet.
   - **Feed back into scan** — after controller phase, feed newly-discovered IPs back into the network scan pipeline for a targeted deep scan pass. This is more complex (requires re-entering Phase 2 partially) but provides complete data in a single session.
   - **Targeted mini-scan** — controller phase triggers a lightweight scan of just the IPs it discovered, running port scan + service matching + per-host integrations. Simpler than re-entering Phase 2 but duplicates some orchestration logic.

   **Recommendation:** Start with "accept the limitation." Most controller-discovered devices (Ubiquiti APs, vSphere VMs) are on scanned subnets and will have been discovered by the network scan already — the controller just enriches them. For devices only reachable via the controller API, the controller's own data (model, firmware, status) is usually sufficient. Revisit if users report missing data on controller-only devices.

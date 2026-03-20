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
#[async_trait]
trait DiscoveryIntegration: Send + Sync {
    /// Which credential type this integration handles
    fn credential_type(&self) -> CredentialTypeDiscriminants;

    /// Integration category determines when/how it runs
    fn category(&self) -> IntegrationCategory;

    /// Run discovery for a single host during deep_scan_host.
    /// Called after port scanning + service matching, when the associated
    /// service is detected AND a matching credential is available.
    /// Returns enrichment data to merge into the host entity.
    async fn discover_for_host(
        &self,
        ctx: &HostIntegrationContext<'_>,
    ) -> Result<HostIntegrationResult, Error>;

    /// Run discovery as a controller integration (e.g., Ubiquiti, vSphere).
    /// Called during the controller phase for credentials targeting
    /// management endpoints. Returns discovered entities.
    async fn discover_as_controller(
        &self,
        ctx: &ControllerIntegrationContext<'_>,
    ) -> Result<ControllerIntegrationResult, Error> {
        // Default: not a controller integration
        Ok(ControllerIntegrationResult::empty())
    }
}

enum IntegrationCategory {
    /// Adds data to the scanned host (SNMP, SSH).
    /// Runs during deep_scan_host after port scan.
    HostEnrichment,

    /// Discovers additional entities from a host (Docker on remote hosts).
    /// Runs during deep_scan_host after port scan.
    EntityDiscovery,

    /// Queries a management endpoint that returns data about many devices.
    /// Runs in a dedicated controller phase, not per-host.
    Controller,

    /// Special: runs inline during port scanning because its data
    /// expands the scan scope. Only SNMP uses this.
    ScanEnrichment,
}
```

### Integration Dispatch Flow

The integration dispatch replaces the current hardcoded extraction in `unified.rs`:

```
credential_mappings (from server)
    ↓
For each CredentialTypeDiscriminants:
    ↓
IntegrationRegistry::get(discriminant) → Box<dyn DiscoveryIntegration>
    ↓
Match on category:
    ScanEnrichment  → passed to NetworkScanDiscovery (runs inline in deep_scan_host)
    HostEnrichment  → collected for post-scan dispatch in deep_scan_host
    EntityDiscovery → collected for post-scan dispatch in deep_scan_host
    Controller      → collected for controller phase
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
    For each credential with ip_override targeting 127.0.0.1:
        Run integration.discover_for_host() with localhost context
    (Docker on localhost is just one case — any localhost credential runs here)

Phase 2: Network Scan
├── ARP discovery (unchanged)
└── deep_scan_host per IP:
    ├── TCP/UDP port scanning (unchanged)
    ├── SNMP inline (ScanEnrichment — unchanged, still special-cased)
    ├── Endpoint scanning (unchanged)
    ├── Service matching (unchanged)
    └── Post-scan integrations:
        For each matched service with an associated credential:
            Run integration.discover_for_host() or discover_for_host()
        (Docker on remote hosts, SSH, future integrations)

Phase 3: Controller
    For each Controller-category credential:
        Run integration.discover_as_controller()
    (Ubiquiti, vSphere — query management endpoints, return multi-device data)
```

### Progress Allocation

Extend `PhaseAllocations` to handle the controller phase:

```
Phase 1 (Daemon Host):    0–5%   (same as current self-report + docker)
Phase 2 (Network Scan):   5–90%  (absorbs current docker-on-localhost time into network)
Phase 3 (Controller):     90–100% (new — API calls are fast, low allocation)
```

When no controller credentials exist, Phase 3 is skipped and Phase 2 gets 5–100%.

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

SNMP is modeled as `IntegrationCategory::ScanEnrichment` — a category that only SNMP currently occupies. It's passed directly to `NetworkScanDiscovery` as today. The `DiscoveryIntegration` trait is not used for SNMP dispatch; instead, SNMP remains a parameter of `NetworkScanDiscovery::new()`.

This is an intentional asymmetry. The trait-based dispatch exists for integrations that run *after* scanning. SNMP's inline nature means it's structurally different — it's part of the scan, not a consumer of scan results.

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
// In deep_scan_host, after service matching:
if services.iter().any(|s| s.base.service_definition.id() == Docker.id()) {
    if let Some(docker_cred) = credential_mappings
        .get_credential_for_ip::<DockerProxy>(&ip)
    {
        let docker_client = new_remote_docker_client(ip, docker_cred)?;
        let containers = docker_runner.scan_containers(&docker_client).await?;
        // Create container hosts as children of this host
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

### Ubiquiti (Controller)

```rust
struct UnifiIntegration;

#[async_trait]
impl DiscoveryIntegration for UnifiIntegration {
    fn credential_type(&self) -> CredentialTypeDiscriminants {
        CredentialTypeDiscriminants::UnifiApi
    }

    fn category(&self) -> IntegrationCategory {
        IntegrationCategory::Controller
    }

    async fn discover_as_controller(
        &self,
        ctx: &ControllerIntegrationContext<'_>,
    ) -> Result<ControllerIntegrationResult, Error> {
        // 1. Connect to UniFi controller at credential's target IP
        // 2. GET /api/s/{site}/stat/device — returns all adopted devices
        // 3. For each device: create Host + interfaces from device.ip, device.mac
        // 4. Map device.type to services (UAP → WiFi AP, USW → Switch, etc.)
        // 5. Return ControllerIntegrationResult with hosts, interfaces, services
    }
}
```

**Credential type:** `UnifiApi { host, port, username, password, site }` — targets the controller endpoint.
**ServiceDefinition:** "UniFi Controller" — matched when the UniFi web UI port (8443) is detected.
**Trigger:** Controller-category credentials run in Phase 3, not triggered by per-host service detection.

### Proxmox / vSphere (Controller)

```rust
struct ProxmoxIntegration;

impl DiscoveryIntegration for ProxmoxIntegration {
    fn credential_type(&self) -> CredentialTypeDiscriminants {
        CredentialTypeDiscriminants::ProxmoxApi
    }

    fn category(&self) -> IntegrationCategory {
        IntegrationCategory::Controller
    }

    async fn discover_as_controller(&self, ctx: ...) -> Result<...> {
        // 1. Connect to Proxmox API at https://{host}:8006/api2/json
        // 2. GET /nodes — list hypervisor nodes
        // 3. GET /nodes/{node}/qemu — list VMs per node
        // 4. For each VM: create Host with virtualization=Proxmox
        // 5. GET /nodes/{node}/lxc — list containers
        // 6. Return all discovered entities
    }
}
```

vSphere follows the same pattern but uses the vSphere API (SOAP/REST) to enumerate VMs and ESXi hosts.

### SSH (Host Enrichment)

```rust
struct SshIntegration;

impl DiscoveryIntegration for SshIntegration {
    fn credential_type(&self) -> CredentialTypeDiscriminants {
        CredentialTypeDiscriminants::Ssh
    }

    fn category(&self) -> IntegrationCategory {
        IntegrationCategory::HostEnrichment
    }

    async fn discover_for_host(
        &self,
        ctx: &HostIntegrationContext<'_>,
    ) -> Result<HostIntegrationResult, Error> {
        // 1. SSH to host using credential (key or password)
        // 2. Run discovery commands: uname, hostnamectl, systemctl list-units
        // 3. Parse output → enrich host with OS info, running services
        // 4. Return HostIntegrationResult with enrichment data
    }
}
```

**Credential type:** `Ssh { username, auth: KeyOrPassword }`.
**ServiceDefinition:** "SSH" — matched when port 22 is open.
**Trigger:** SSH service detected on host → SSH credential available → `discover_for_host()` runs post-scan.

### Aruba (Controller)

Same pattern as Ubiquiti — controller integration querying Aruba Central API or AOS-CX REST API for switch/AP topology data.

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
1. Define `DiscoveryIntegration` trait and `IntegrationCategory` enum in `daemon/discovery/integrations/mod.rs`
2. Create `IntegrationRegistry` — maps `CredentialTypeDiscriminants` → `Box<dyn DiscoveryIntegration>`
3. Implement `DockerIntegration` using the trait
4. Refactor `deep_scan_host()` to call integrations after service matching:
   - Get matched services → find associated credentials → dispatch to integration
5. Refactor `run_unified_phases()` to collect and dispatch integrations by category

**Dependencies:** Phase 1 (remote Docker provides the parameterized scanning logic to wrap in the trait).

### Phase 3: Controller Phase

**Goal:** Add the controller phase to `unified.rs` for management-endpoint integrations.

**Changes:**
1. Extend `PhaseAllocations` with controller phase
2. Add `run_controller_phase()` to `DiscoveryRunner<UnifiedDiscovery>` — iterates controller-category integrations, calls `discover_as_controller()`
3. Define `ControllerIntegrationContext` and `ControllerIntegrationResult` types
4. Controller results create hosts/services/interfaces via existing `create_host()` infrastructure

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

**Dependencies:** Phase 2 for host integrations, Phase 3 for controller integrations.

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

3. **Integration result merging.** When `discover_for_host()` returns enrichment data (SSH OS info, for example), how does it merge into the host entity? Options:
   - Return a partial `HostBase` with only populated fields → merge before `create_host()`
   - Return a structured `HostEnrichment` type → host creation logic merges it
   - Modify the host in-place via a mutable reference

   **Recommendation:** Return `HostIntegrationResult` with optional fields, merge before `create_host()` (same pattern as SNMP enrichment today).

4. **Multiple credentials of the same type.** If two `DockerProxy` credentials target different hosts, `build_credential_mappings_for_discovery()` already handles this via `ip_overrides`. But what if two credentials of the same type target the same host? Currently only the first `ip_override` match wins. Is this sufficient?

   **Recommendation:** Yes — one credential per type per host is the intended model. The UI should prevent duplicate assignments.

5. **Error isolation.** If a post-scan integration fails for one host (e.g., Docker connection refused), should it:
   - Fail silently (log + continue) — current Docker behavior
   - Fail the host (skip creating it)
   - Fail the entire phase

   **Recommendation:** Fail silently with structured error logging. Integration failures should not block host creation — the host was already discovered via port scanning. Surface integration errors in the discovery session summary.

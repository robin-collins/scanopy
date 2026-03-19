# Docker Discovery UX Review

## 1. Current State

### Local Docker Socket

**Data flow:** `scan_local_docker_socket` is a boolean field on `DiscoveryType::Unified` (`server/discovery/impl/types.rs`). In the UI, it appears as a checkbox in `DiscoveryTargetsForm.svelte:149-182`, visible only when the discovery type is Unified.

**Auto-population:** When creating a discovery, the checkbox is auto-populated from `daemon.capabilities.has_docker_socket`. The daemon detects socket availability at startup and on every heartbeat via `detect_docker_socket()` (`daemon/runtime/service.rs:321-329`).

**Auto-created discoveries:** `create_default_discovery_jobs()` in `server/daemons/service.rs:1075-1138` sets `scan_local_docker_socket: true` if the daemon reports socket available.

**Graceful degradation:** If the socket is unavailable at scan time, the daemon warns and skips the Docker phase — no error surfaced to the user.

### DockerProxy Credentials

**Credential type:** `CredentialType::DockerProxy` — fields: port (default 2376), optional path, SSL cert/key/chain.

**Scope:** `PerHost` only.

**User journey:** Settings -> Credentials -> Create -> Host Edit -> Credentials tab -> Assign -> Run discovery. This is 6 steps across multiple pages and modals. There is no visibility in the discovery modal about whether any hosts have DockerProxy credentials assigned.

### Daemon Install & Host Creation

**Wizard flow:** Configure -> Install -> Advanced (`CreateDaemonModal.svelte`).

**ServerPoll:** `provision_daemon()` creates Host + Daemon + ApiKey before the daemon installs. The `provisionedDaemonId` is tracked (line 236) for later reference.

**DaemonPoll:** Host is created during `process_registration()` when the daemon first connects, with empty `credential_assignments`.

**Install command:** Built client-side from field definitions via `buildRunCommand()` (`utils.ts:86-156`); Docker Compose built via `buildDockerCompose()` (`utils.ts:158+`).

**Note:** `--docker-proxy` CLI flags on the daemon are being deprecated and should be ignored.

### Timing Problem

Daemon auto-detects socket -> reports `has_docker_socket: true` -> server auto-creates discovery with `scan_local_docker_socket: true` -> scanning starts immediately. **The user has no way to disable Docker scanning before the first scan runs.**

---

## 2. UX Gaps

1. **DockerProxy is invisible.** New users won't know it exists. The only hint is a small DocsHint below the socket checkbox in the discovery modal.

2. **Disconnected mental model.** The local socket is a checkbox in the discovery modal; Docker proxy is configured across two separate settings pages (Credentials + Host edit). These feel like unrelated features despite both being "Docker discovery."

3. **No pre-install Docker control.** The daemon starts scanning Docker immediately on first run. There is no opt-out before the first scan.

4. **No feedback in discovery modal.** No indication in the discovery creation/edit UI about whether the daemon's host has a DockerProxy credential assigned.

5. **Docker errors are backend-only.** Connection failures are logged server-side but never surfaced in the UI.

### Broadcast Scope for DockerProxy -- Not Needed

Considered adding `Broadcast` scope so a single DockerProxy credential covers all hosts on a network. The real use case is narrow (uniform Docker Swarm/k8s clusters where every node has identical API config). The daemon install flow solves the primary gap more directly. For other hosts, per-host assignment is appropriate since different hosts likely have different Docker API configurations.

---

## 3. Improvement Proposals

### P1: Docker Proxy Indicator in Discovery Modal

**Impact:** Medium. **Effort:** Small.

Add a Docker proxy status indicator below the socket checkbox in the discovery modal:

- **If daemon's host has a DockerProxy credential:** Show EntityRef to the credential + EntityRef to the daemon's host. Disable the socket checkbox with an explanation that the proxy overrides local socket scanning.
- **If no DockerProxy credential on daemon's host:** Show an "[Add Docker proxy credential]" link that opens the credential creation modal.

This closes the "invisible DockerProxy" gap without changing any flows.

### P2: Docker Config in Daemon Advanced Settings

**Impact:** High. **Effort:** Medium.

Add a Docker section to the Advanced tab of the daemon creation wizard (`AdvancedStep.svelte`).

#### UI Design

A segmented control with three options:
- **Disabled** -- no Docker discovery
- **Local Socket** (default) -- daemon scans its own host's Docker socket
- **Proxy** -- inline credential creation form appears

When **Proxy** is selected, an inline credential creation form appears within the Advanced tab. This form is a reusable, embeddable version of the credential creation form, abstracted to accept `credentialType` as a parameter (the type dropdown is hidden -- it's always DockerProxy). This abstraction opens the door to embedding credential creation forms in other contexts for any credential type.

#### After Credential Creation

Once the user saves the inline credential form and the credential is created via API, the segmented control locks -- Disabled and Local Socket options become disabled. The inline form is replaced with a confirmation message (EntityRef to the created credential + "Docker proxy credential created"). This prevents switching modes after a credential has been created, avoiding orphaned credentials. The install command / Docker Compose updates reactively to include the credential ID.

**Credential naming:** Auto-name using the daemon name (e.g., `"my-daemon"`). The credential type is already displayed separately in the UI.

#### Simplification: No New Server-Side Fields

**`enable_local_docker_socket` folds into `has_docker_socket`:** The new daemon config flag gates `detect_docker_socket()` itself. If config says disabled, daemon reports `has_docker_socket: false` regardless of physical socket presence. Server logic doesn't change -- it still uses `has_docker_socket` exactly as before for `create_default_discovery_jobs()` and UI auto-population.

This gives users two clear controls:
1. **Daemon config** (set at install, baked into CLI/env): whether the daemon reports Docker socket availability
2. **Discovery toggle** (existing checkbox): whether to scan Docker on a given discovery run

Daemon config gates the *default*. Discovery toggle is the ongoing control. No three-way complexity.

**Daemon logging:** If `enable_local_docker_socket` is false, daemon logs "Local Docker socket scanning disabled by configuration" at startup. If socket detection fails (when enabled), existing warning log covers it.

**`docker_proxy_credential_id` travels via registration:** Daemon includes credential ID in `DaemonRegistrationRequest` (new optional field). Server, during `process_registration()` after host creation, calls `set_host_credentials()` to assign the credential. One-time action -- credential then flows through normal `build_credential_mappings()` pipeline.

#### Backend Changes

- **Daemon CLI** (`daemon/shared/config.rs`): Two new AppConfig fields -- `enable_local_docker_socket: bool` (default true), `docker_proxy_credential_id: Option<Uuid>`
- **`detect_docker_socket()`**: Early-return false if `enable_local_docker_socket` is false
- **`DaemonRegistrationRequest`**: Add `docker_proxy_credential_id: Option<Uuid>`
- **`process_registration()`**: If credential ID provided, assign to host after creation via `set_host_credentials()`
- No changes to `ProvisionDaemonRequest` -- for ServerPoll, frontend assigns credential to host via existing host update API after user completes Advanced tab

#### Frontend Changes

- **`AdvancedStep.svelte`**: Docker section with 3-way segmented control
- Abstract credential creation form into embeddable component (accepts `credentialType` param, hides type selector)
- **`buildRunCommand()`**: Include `--enable-local-docker-socket false` for Disabled/Proxy, include `--docker-proxy-credential-id <uuid>` for Proxy
- **`buildDockerCompose()`**: Include equivalent env vars (`SCANOPY_ENABLE_LOCAL_DOCKER_SOCKET`, `SCANOPY_DOCKER_PROXY_CREDENTIAL_ID`)

#### Handling the Timing Constraint

`provision_daemon()` (ServerPoll) is called on Configure -> Install transition (`handleCreateNewApiKey()` at `CreateDaemonModal.svelte:230`), **before** the user sees Advanced. Docker config in Advanced means the credential doesn't exist at provision time.

**Flow for Proxy selection:**
1. User selects "Proxy" in Advanced tab
2. Inline form: port (default 2376), optional SSL cert/key/chain
3. Frontend creates DockerProxy credential via normal credential API -> gets credential ID
4. **DaemonPoll**: Credential ID baked into install command (`--docker-proxy-credential-id <uuid>`) and Docker Compose env var (`SCANOPY_DOCKER_PROXY_CREDENTIAL_ID`). Daemon passes it in `DaemonRegistrationRequest`. Server assigns to host during `process_registration()` after host creation.
5. **ServerPoll**: Host already exists from `provision_daemon()`. After user creates credential in Advanced tab, frontend assigns credential to the provisioned daemon's host via host update API (`UpdateHostRequest.credential_assignments`). The `provisionedDaemonId` is already tracked (line 236) so we can look up the host.

The DaemonPoll path also needs `--enable-local-docker-socket false` in the install command when Proxy or Disabled is selected. The install command is rebuilt reactively whenever form values change, so changes in Advanced are reflected before the user copies it.

**Why this works without special-casing:** The credential is created through the normal credential API. Assignment uses existing `set_host_credentials()`. Discovery uses existing `build_credential_mappings()` to send credentials to the daemon. No new credential resolution logic needed.

---

## 4. Recommendations Summary

| Priority | Proposal | Impact | Effort |
|----------|----------|--------|--------|
| 1 | **P2**: Docker section in daemon Advanced tab | High | Medium |
| 2 | **P1**: Docker proxy indicator in discovery modal | Medium | Small |

**P2 first** because it eliminates the 6-step journey for Docker proxy setup, gives users control over first-scan Docker behavior, and creates the reusable embeddable credential form component that P1 can also use.

**P1 second** because it closes the feedback gap in the discovery modal and benefits from the embeddable credential form built in P2.

---

## 5. Notes for Documentation

The relationship between daemon config (`enable_local_docker_socket`) and the discovery toggle (`scan_local_docker_socket`) should be documented:

- **Daemon config** controls whether the daemon *reports* Docker availability
- **Discovery toggle** controls whether a specific discovery *scans* Docker
- If daemon config disables Docker, the discovery toggle defaults to off and scanning is skipped even if manually enabled
- This is analogous to how physical socket absence works today -- the mechanism is the same, just user-configurable

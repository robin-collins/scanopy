# Phase 3: Unified Discovery Model + Full Daemon Credentials

## Status

Started: 2026-03-17

## Worktree Plan (5 worktrees)

```
W1 (Unified Type + Version Gating + Migration)  ──┐
                                                    ├── W3 (Daemon Handler)
W2 (Generic Credential Wire Format)               ─┘

W4 (Frontend) ── depends on W1
W5 (Documentation) ── depends on W1, W4
```

**Merge order:** W1 → W2 → W3 → W4 → W5

## Worktree Status

| # | Branch | Path | Status |
|---|--------|------|--------|
| W1 | `refactor/unified-discovery-type` | `scanopy-unified-discovery-type` | Ready to launch |
| W2 | `refactor/generic-credential-dispatch` | `scanopy-generic-credential-dispatch` | Ready to launch |
| W3 | `refactor/daemon-unified-handler` | TBD | Blocked on W1+W2 merge |
| W4 | `refactor/unified-discovery-frontend` | TBD | Blocked on W1 merge |
| W5 | `docs/unified-discovery-migration` | TBD | Blocked on W1+W4 merge |

## Initiation Commands

**W1:**
```bash
cd /Users/maya/dev/scanopy-unified-discovery-type && claude "Read /Users/maya/dev/scanopy-unified-discovery-type/TASK.md and /Users/maya/dev/scanopy-unified-discovery-type/CLAUDE.md, then begin work."
```

**W2:**
```bash
cd /Users/maya/dev/scanopy-generic-credential-dispatch && claude "Read /Users/maya/dev/scanopy-generic-credential-dispatch/TASK.md and /Users/maya/dev/scanopy-generic-credential-dispatch/CLAUDE.md, then begin work."
```

## Goals

- Single unified discovery type replaces three legacy types (SelfReport, Network, Docker)
- Version-gate discovery creation AND registration (legacy daemons can't register or create discoveries)
- Auto-migrate on daemon upgrade (archive old, create unified)
- Generic credential dispatch to daemon (not just SNMP)
- Docker proxy via credential store (AppConfig deprecated with warnings, removed in v0.16.0)
- ScanSettings moves into Unified variant (removed from DiscoveryBase and DaemonDiscoveryRequest top level)

---

## W1: Unified Discovery Type + Version Gating + Migration

### Scope
- Add `DiscoveryType::Unified` variant with `host_id`, `subnet_ids`, `scan_local_docker_socket`, `host_naming_fallback`, `scan_settings`
- Move `scan_settings` from `DiscoveryBase` and `DaemonDiscoveryRequest` into `Unified` variant only
- Add `is_legacy()` helper
- Version gating: `MINIMUM_UNIFIED_DISCOVERY = 0.15.0`, `supports_unified_discovery()` helper, field on `DaemonVersionStatus`
- Discovery creation: reject legacy types (400), reject Unified for unsupported daemon (400)
- Daemon registration: reject < 0.15.0, create one Unified discovery for new daemons
- Migration: `migrate_discoveries_to_unified(daemon_id)` — triggered on re-registration and ServerPoll version change
- Session hydration: legacy Network keeps SNMP embedding, Unified passes type through unchanged

### Key Files
- `server/discovery/impl/types.rs` — Unified variant
- `server/discovery/impl/base.rs` — remove scan_settings
- `server/daemons/impl/api.rs` — remove scan_settings from request/payload
- `server/daemons/impl/version.rs` — version constants and helpers
- `server/discovery/handlers.rs` — creation validation
- `server/discovery/service.rs` — session hydration
- `server/daemons/service.rs` — migration, registration, ServerPoll detection

---

## W2: Generic Credential Wire Format

### Scope
- New types: `CredentialQueryPayload` (Snmp/DockerProxy), `DockerProxyQueryCredential`, `ResolvableValue`, `ResolvableSecret`
- New `FileOrInline` type for non-secret inline-or-file fields on DockerProxy (replaces `Option<String>` for ssl_cert/ssl_chain)
- Backwards-compatible deserialization for old String format
- `to_query_payload()` on `CredentialType` — compile-time exhaustive conversion
- `discovery_label()` on `CredentialQueryPayload`
- Add `credential_mappings: Vec<CredentialMapping<CredentialQueryPayload>>` to `DaemonDiscoveryRequest`
- `with_exposed_credentials()` method (parallel to `with_exposed_snmp()`)
- `build_credential_mappings_for_discovery(network_id, daemon_host_id)` — generic builder
- DockerProxy gating: only include for daemon's own host

### Key Files
- `server/credentials/impl/mapping.rs` — new payload types
- `server/credentials/impl/types.rs` — FileOrInline, to_query_payload()
- `server/credentials/service.rs` — generic builder
- `server/daemons/impl/api.rs` — credential_mappings field

---

## W3: Daemon Unified Discovery + Capabilities

### Scope
- New `UnifiedDiscovery` handler in `daemon/discovery/service/unified.rs`
- Legacy types → stubs (log warning, complete immediately)
- Phase flow: self-report (5%) → network (80%) → docker (15%)
- Docker proxy: credential_mappings > AppConfig; deprecation warning per run
- Credential logging with masking and discovery labels
- ResolvableValue/Secret resolution utility in `daemon/utils/`
- Capability re-detection every 5 min

### Key Files
- `daemon/discovery/service/unified.rs` (NEW)
- `daemon/discovery/manager.rs`
- `daemon/discovery/service/network.rs`, `docker.rs`, `self_report.rs`
- `daemon/runtime/service.rs`
- `daemon/shared/config.rs`

---

## W4: Frontend

### Scope
- Remove discovery type selector
- Unified creation form: subnet picker, Docker socket toggle, host naming fallback, scan settings (Speed tab)
- Version warnings: daemon upgrade modal, scheduled discovery page, legacy discovery labels
- If daemon doesn't support unified: disable submit + show upgrade warning

### Key Files
- `ui/src/lib/features/discovery/components/DiscoveryModal/`
- `ui/src/lib/features/daemons/components/DaemonUpgradeModal.svelte`
- `ui/src/lib/features/daemons/components/DaemonCard.svelte`

---

## W5: Documentation

### Scope
- Migration guide: what happens on daemon upgrade
- Docker proxy migration guide: AppConfig → credential store
- Audit existing docs for old-model references

### Additional docs needed
- **Host consolidation + credentials**: Document that host consolidation now migrates credential assignments. When merging hosts, host-specific credential overrides from the source host are preserved on the destination (unless the destination already has the same credential assigned). This is a behavior change — previously, credentials were silently lost.

### User Action Required (highlight in docs)
Users upgrading to v0.15.0 need to take action if:
1. **Docker proxy configured in daemon config** — must recreate as a DockerProxy credential in the UI. Daemon config values (`docker_proxy`, `docker_proxy_ssl_cert`, `docker_proxy_ssl_key`, `docker_proxy_ssl_chain`) are deprecated and will be removed in v0.16.0.
2. **Overridden scan speed settings in daemon config** — values like `arp_retries`, `arp_rate_pps`, `scan_rate_pps`, `port_scan_batch_size` are now per-discovery (in the Speed tab of the discovery modal). Users who customized these must re-apply their values in the discovery's scan settings. Daemon-level values are ignored.

Docs URL: `https://scanopy.net/docs/guides/unified-discovery-migration/`

---

## Verification Checklist (post-merge)

1. New daemon registration (>= 0.15.0): creates one unified discovery
2. Legacy daemon registration (< 0.15.0): rejected with upgrade error
3. Daemon upgrade (DaemonPoll): process_registration() triggers migration
4. Daemon upgrade (ServerPoll): version change detected, triggers migration
5. Migration: old discoveries → Historical records, unified created
6. Unified execution: self-report → network → docker (5/80/15% progress)
7. ScanSettings inside Unified variant, configurable in modal Speed tab
8. Generic credentials: SNMP + DockerProxy via credential_mappings
9. Docker proxy: credential_mappings > AppConfig; deprecation warning
10. Docker proxy gating: only sent for daemon's own host
11. Credential logging: masked secrets, discovery labels
12. Capability re-detection: periodic, decoupled from scanning
13. Backwards compat: old daemons read Network.snmp_credentials, ignore credential_mappings
14. `cargo test --lib`, `npm test`, `make format && make lint` all pass

# Scanopy Collector Expansion Task List

This is the live execution and handoff ledger for the collector work described in
`SOL-STATE.md`. Update checkboxes and the evidence log as work completes. Never record passwords,
private keys, SNMP secrets, enrolment tokens, live credential values, or customer command output.

## Environment

- Development repository: `C:\projects\scanopy`
- Reference CodexNet repository: `C:\projects\codexnet`
- Development OS: Windows
- Deployment host: `osit@100.69.66.108`
- Deployment directory: `/opt/scanopy`
- Branch: `forkThat`
- SSH release commits: `479b9b28c`, `0252b30cb`, `e1623c8f9`
- Currently deployed revision: `0b419b6f6`

## Status Legend

- `[x]` complete and evidenced
- `[ ]` outstanding
- `[~]` in progress
- `[!]` blocked; the reason must be recorded in **Blockers and decisions**

## 1. Existing Fork and LLDP Baseline

- [x] Remove Community/MSP network-count gates.
- [x] Restore network creation and naming UI/API flows.
- [x] Publish fork server and daemon images through GHCR.
- [x] Deploy the fork images from `/opt/scanopy`.
- [x] Fix LLDP discovery for high SNMP interface indexes (`64f23d45e`).
- [x] Correct the daemon configuration volume and persist daemon enrolment.
- [x] Verify a fresh scan and populated L2 topology.

## 2. SSH Vertical Slice

### 2.1 Foundation

- [x] Add password and private-key SSH credential transports.
- [x] Add platform, port, host-key policy, and `known_hosts` configuration.
- [x] Reuse secret resolution, redaction, and redacted update behavior.
- [x] Add native Rust/Russh daemon integration.
- [x] Add fixed read-only Linux, Cisco IOS, Comware, and ArubaOS-Switch allowlists.
- [x] Add timeouts, cancellation checks, output bounds, and secret-free probe errors.
- [x] Add standard/custom SSH port probing and service matching.
- [x] Add CodexNet provenance record.

### 2.2 Compatibility and generated artifacts

- [x] Select and apply an intentional fork version and daemon compatibility floor (`0.18.0`).
- [x] Run the `make generate-fixtures` recipe successfully (direct commands; GNU Make unavailable).
- [x] Run the `make generate-types` recipe successfully (direct commands; GNU Make unavailable).
- [x] Confirm both SSH credential schemas appear in generated TypeScript/OpenAPI artifacts.
- [x] Confirm `SshPassword` and `SshPrivateKey` appear in the database enum baseline fixture.
- [x] Review all generated metadata, translations, schemas, and fixtures for unintended changes.

### 2.3 Automated tests

- [x] Test non-empty command allowlists.
- [x] Test UTF-8-safe truncation.
- [x] Test standard/custom port conversion.
- [x] Add service matching regression tests for port and client-response paths.
- [x] Add a mock SSH server test harness.
- [x] Test strict host-key acceptance and rejection.
- [x] Test password acceptance and rejection.
- [x] Test private-key authentication, including passphrase handling.
- [x] Test command timeout and cancellation, including mid-command cancellation.
- [x] Test bounded oversized output handling.
- [x] Test that errors and debug output do not expose secrets.
- [x] Test hostname/system-description enrichment.
- [x] Test probe-level connection/authentication timeout and cancellation.
- [x] Test rejection when every allowlisted command fails.
- [x] Test bounded/validated hostname persistence and SSH secret-file resolution.
- [x] Test daemon-platform `known_hosts` path validation on Windows and Unix syntax.

### 2.4 Functional behavior and UX

- [x] Document persistent `known_hosts` mounting/bootstrap behavior.
- [x] Decide whether fingerprint pinning or explicit TOFU is required for this slice.
- [ ] Validate Linux enrichment against an authorized disposable host.
- [!] Validate exec-channel and paging behavior on authorized Cisco IOS equipment.
- [!] Validate exec-channel and paging behavior on authorized Comware equipment.
- [!] Validate exec-channel and paging behavior on authorized ArubaOS-Switch equipment.
- [!] Verify lab accounts cannot enter configuration mode.
- [x] Decide and document which normalized interface/VLAN/MAC/ARP/LLDP/CDP/inventory/PoE/
  environment fields belong in Scanopy models.
- [x] Ensure unrestricted raw CLI output is not persisted by default.
- [x] Fix dynamic credential fields requiring a delete/retype before Save.
- [x] Accept OpenSSH private-key envelopes in the credential form.

### 2.5 Dependency and build review

- [x] Run an exact-lock RustSec/OSV audit and resolve or document SSH-related findings.
- [x] Review new dependency licences and Russh's transitive graph.
- [x] Regression-test the `base64ct` upgrade against authentication and email code.
- [x] Build/check the backend on the Windows development host.
- [x] Build Linux musl AMD64 images.
- [x] Build Linux musl ARM64 images.
- [x] Assess image-size impact against the deployed baseline.

### 2.6 Quality gates and deployment

- [x] `cargo fmt --check` passes.
- [x] Focused SSH tests pass (12 SSH integration tests and 2 service-matching tests).
- [x] Credential tests pass (62 tests).
- [x] `cargo clippy --lib -- -D warnings` passes.
- [x] `make test-unit` passes (Windows-equivalent commands; GNU Make unavailable).
- [x] `make lint` passes, including migration lint (Windows-equivalent commands).
- [x] `make test` passes (Windows-equivalent commands; clean Compose state).
- [x] Review the final diff and split changes into logical commits.
- [x] Push reviewed SSH release commits to `forkThat`.
- [x] Publish both server and daemon images to GHCR.
- [x] Update the Compose deployment on `osit@100.69.66.108:/opt/scanopy`.
- [~] Publish and deploy the SSH credential-form hotfix.
- [ ] Verify a deployed credentialed scan without exposing credentials.
- [ ] Verify existing SNMP/LLDP discovery has not regressed.

## 3. Active Directory Slice

- [ ] Define native AD entity mapping and bounded persistence.
- [ ] Add LDAPS password credential transport with secure TLS defaults.
- [ ] Add Kerberos/system-ccache transport.
- [ ] Collect approved domain, forest, site, subnet, controller, trust, computer, and explicitly
  configured group membership data.
- [ ] Explicitly exclude password/LAPS material, credential dumping, roasting, and attack-path
  collection.
- [ ] Add mock, focused, UI/schema, integration, and redaction tests.
- [ ] Build, publish, deploy, and verify the AD slice before starting UniFi.

## 4. UniFi Slice

- [ ] Define controller URL, site, TLS, password, and supported token credential fields.
- [ ] Default TLS verification on; make any self-signed exception endpoint-specific and explicit.
- [ ] Map controller data into existing hosts, interfaces, and topology where possible.
- [ ] Add mock, focused, UI/schema, integration, and redaction tests.
- [ ] Build, publish, deploy, and verify the UniFi slice before passive collectors.

## 5. Passive Observation Slice

- [ ] Design daemon-level mDNS, DHCP, and neighbor ingestion lifecycle outside the credential
  integration registry.
- [ ] Define bounded structured observations, provenance, confidence, retention, and correlation.
- [ ] Implement mDNS ingestion and tests.
- [ ] Implement DHCP observation ingestion and tests.
- [ ] Implement neighbor observation ingestion and tests.
- [ ] Build, publish, deploy, and verify passive collection.

## Blockers and Decisions

- Authorized real network-device targets and suitable read-only test accounts must be identified
  before the three device-family validation items can complete.
- The remote inventory has no authorized Cisco IOS, Comware, or ArubaOS-Switch lab targets, so
  those hardware/channel/paging checks cannot be completed without targets and read-only accounts.
- Do not print or copy live daemon or discovery credentials during remote verification.
- The remote Compose file uses mutable `:forkthat` image tags. Before each deployment, verify the
  tag digest matches the intended immutable revision tag, then pull and recreate deliberately.

## Evidence Log

- `a68a69f40`: SSH collector foundation and handoff committed on `forkThat`.
- `64f23d45e`: LLDP high-index fix documented as deployed and user-verified.
- Handoff evidence: `cargo check --lib` passed and `cargo fmt` was applied before `a68a69f40`.
- Coordinated package version set to `0.18.0`; the SSH credential floor remains `0.18.0` so older
  daemons cannot receive the new SSH wire variant.
- `docs/SSH_DISCOVERY.md`: strict host-key bootstrap, no-TOFU decision, bounded persistence, and
  normalization boundary documented for the initial slice.
- Remote read-only check on 2026-07-18: Compose services were healthy on ARM64 and server/daemon
  image labels reported deployed revision `64f23d45e`.
- `docs/SSH_DEPENDENCY_REVIEW.md`: exact-lock advisory/licence review, scoped RSA advisory
  assessment, `base64ct` contracts, and multi-architecture/image-size evidence recorded.
- GitHub Actions run `29627966115` built/published the preliminary `a68a69f40` server amd64/arm64
  and daemon musl amd64/arm64 images; the later release run is recorded below.
- Windows fixture generation passed using the Npcap SDK 1.13 `Lib\x64` path. Meta-message sync
  passed after applying upstream `158da0400`'s removal of the stale `group-types.json` entry.
- Generated OpenAPI contains 151 full/100 public paths and 283 schemas; the reviewed schema delta
  adds only `SshHostKeyPolicy`, `SshPlatform`, the two credential variants, and version `0.18.0`.
- DB enum baseline regeneration passed with 18 catalogued enums and 9 credential variants.
- Loopback Russh verification passed 9/9 tests; SSH service matching passed 2/2 tests.
- `cargo test --lib server::credentials` passed 62/62 tests; the two direct `base64ct` contract
  tests passed.
- Post-review SSH hardening adds a 20-second probe deadline/cancellation, rejection of total
  command failure, validated 253-byte hostnames, cross-platform strict-path validation, and a
  64 KiB SSH secret limit. The expanded loopback SSH suite passes 12/12.
- Windows frontend gates pass: 12 Vitest files/91 tests, Prettier, ESLint, and Svelte check with
  zero errors/warnings. Repository LF policy and generated Paraglide ignores make this repeatable.
- Backend library verification passed 534 tests plus the 3 compatibility filters (6 ignored);
  PostgreSQL fixture restoration used an ephemeral containerized `psql` wrapper because only WSL
  had the client installed.
- Strict Clippy passes for the library, server binary, and daemon binary. Squawk migration lint
  passes for all 21 in-scope migrations (14 transactional, 3 non-transactional, 4 exceptions).
- OpenAPI full/public generation is byte-for-byte stable across consecutive runs after making
  documentation bindings, services, timestamps, and default topology-rule IDs deterministic.
- Final review regressions pass: nonzero SSH command exits fail without persisting stderr, the
  combined stdout/stderr limit remains 1 MiB, and alternate SSH ports are gated per credential.
- The canonical `v0.18.0` compatibility fixture set contains 45 daemon-to-server exchanges and 7
  deduplicated server-to-daemon exchanges; its ephemeral local-test key was invalidated when the
  clean integration stack was destroyed.
- Clean-state Docker integration passed 5/5 in 648.95 seconds: DaemonPoll and ServerPoll discovery,
  API compatibility replay, CRUD, billing, validation, permissions, and daemon lifecycle. The
  harness removed its containers, local images, networks, and volumes after success.
- GitHub Actions run `29633225850` published release revision `0b419b6f6` for server and daemon on
  linux/amd64 and linux/arm64; both mutable `forkthat` manifests matched the immutable revision
  manifests before deployment.
- Remote deployment on 2026-07-18 is healthy at server/daemon revision `0b419b6f6`; `/api/version`
  reports `0.18.0`, and daemon negotiation reports the same server version.
- A disposable loopback SSH production-path validation was safely removed after a transient API
  connection refusal interrupted the run. The exact temporary container and credential are absent,
  and the one orphaned exact-name API key was deleted and verified at zero rows.
- SSH credential-form regressions cover OpenSSH/PKCS#8/RSA/EC envelopes, mismatched envelope
  rejection, untouched default port registration, and untouched edit values. Full UI verification
  passes 14 Vitest files/98 tests, Prettier, ESLint, and Svelte check with zero errors/warnings.

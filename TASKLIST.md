# Scanopy Collector Expansion Task List

This is the authoritative live execution and handoff ledger for the collector work originally
described in `SOL-STATE.md`. It supersedes that historical SSH-era snapshot wherever the two files
conflict. Update checkboxes and the evidence log as work completes. Never record passwords, private
keys, SNMP secrets, enrolment tokens, live credential values, or customer command output.

## Overall Goals

- Preserve the existing fork, network-count, LLDP, enrolment, and SSH discovery baseline.
- Deliver bounded, read-only Active Directory and UniFi collectors with secure credential handling.
- Deliver explicit opt-in passive mDNS, DHCP, and neighbor observation with bounded retention.
- Allow members to manually arrange live topology hosts and services without making the derived
  graph itself mutable, and enrich active discoveries through the daemon host's local DNS resolver.
- Ship one coordinated, security-reviewed `0.19.1` server/daemon release for AMD64 and ARM64.
- Deploy and verify the release safely while recording external-target validations as blockers when
  the required authorization, equipment, credentials, or maintenance window has not been supplied.

## Environment

- Development repository: `C:\projects\scanopy`
- Reference CodexNet repository: `C:\projects\codexnet`
- Development OS: Windows
- Deployment host: `osit@100.69.66.108`
- Deployment directory: `/opt/scanopy`
- Branch: `forkThat`
- SSH release commits: `479b9b28c`, `0252b30cb`, `e1623c8f9`
- Currently deployed revision: `a74e423ad`

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
- [!] Validate Linux enrichment against an authorized disposable host.
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
- [x] Publish and deploy the SSH credential-form hotfix (`0796648bd`).
- [!] Verify a deployed credentialed scan without exposing credentials.
- [!] Verify existing SNMP/LLDP discovery has not regressed.

## 3. Active Directory Slice

- [x] Define native AD entity mapping and bounded persistence.
- [x] Add LDAPS password credential transport with secure TLS defaults.
- [x] Add Kerberos/system-ccache transport and build-capability negotiation.
- [x] Collect approved domain, forest, site, subnet, controller, trust, computer, and explicitly
  configured group membership data.
- [x] Explicitly exclude password/LAPS material, credential dumping, roasting, and attack-path
  collection.
- [x] Add loopback LDAPS, focused, UI/schema, integration, compatibility, and redaction tests.
- [x] Generate and review credential metadata, OpenAPI, TypeScript, fixtures, and translations.
- [x] Prove glibc AMD64 and ARM64 daemon builds with `ad-gssapi` and the read-only ccache contract.
- [x] Build, publish, deploy, and verify the coordinated `0.19.1` collector release.
- [!] Validate LDAPS and Kerberos collection against an authorized real AD/KDC environment.

## 4. UniFi Slice

- [x] Define controller URL, site, TLS, password, and supported authentication fields; document why
  local-controller token authentication is not exposed without a supported read API.
- [x] Default TLS verification on; make any self-signed exception endpoint-specific and explicit.
- [x] Map controller data into existing hosts, interfaces, and topology where possible without
  replacing authoritative SNMP interface data.
- [x] Add real-loopback HTTPS, focused, UI/schema, execute-path, and redaction tests.
- [x] Generate and review credential metadata, OpenAPI, TypeScript, fixtures, and translations.
- [x] Build, publish, deploy, and verify the coordinated `0.19.1` collector release.
- [!] Validate collection against an authorized real UniFi controller.

## 5. Passive Observation Slice

- [x] Design daemon-level mDNS, DHCP, and neighbor ingestion lifecycle outside the credential
  integration registry.
- [x] Make continuous passive collection explicit opt-in and keep `ServerPoll` behavior unchanged.
- [x] Define bounded structured observations, provenance, confidence, retention, and correlation.
- [x] Implement mDNS ingestion and tests.
- [x] Implement DHCP observation ingestion and tests.
- [x] Implement neighbor observation ingestion and tests.
- [x] Verify the database ingestion, deduplication, authorization scope, and retention integration
  tests against a clean PostgreSQL stack.
- [x] Build, publish, deploy, and verify the coordinated `0.19.1` collector release; collection
  remains off by default.
- [!] Validate live capture during an approved passive-observation maintenance window.

## 6. Cross-Slice Release Gates

- [x] Resolve or explicitly document all security/release review findings for AD, UniFi, passive
  observations, image publication, and deployment.
- [x] Run Rust format, strict Clippy, library/binary/unit, migration, and clean integration gates.
- [x] Run frontend Vitest, Svelte type-check, ESLint, and Prettier gates after final generation.
- [x] Review the complete diff and split it into independently revertible logical commits.
- [x] Push `forkThat`, verify the image-publishing workflow, and match mutable/immutable digests.
- [x] Deploy the intended server and daemon revisions, apply migrations, and verify health/version.
- [x] Perform every production-path validation possible without exposing secrets or inventing
  authorized external targets; record unavoidable external blockers precisely.

## 7. Manual Topology Layout and Local DNS

- [x] Persist grid-snapped node positions separately for each topology and view.
- [x] Authorize position writes from the persisted topology network rather than client-supplied
  network data, validate live movable nodes server-side, and cap each view at 10,000 overrides.
- [x] Allow manual movement of service/leaf elements and host containers from an explicit Edit
  mode while keeping snapshots, shares, embeds, resizing, and edge reconnection read-only.
- [x] Ignore stale overrides whose node disappeared or changed parent, and provide a per-view Reset
  action that returns the view to automatic layout.
- [x] Use the daemon host's operating-system/NSS resolver for active-scan IP-to-hostname lookup,
  with cancellation, a two-second deadline, bounded concurrency, and DNS-name normalization.
- [x] Preserve the full valid PTR result as hostname metadata while bounding the display name and
  retaining manual-name and SSH/SNMP precedence behavior.
- [x] Regenerate OpenAPI and TypeScript contracts and add backend/UI regression coverage.
- [x] Pass Rust format and strict Clippy, all 608 Rust library tests (602 passed, 6 ignored),
  Squawk migration lint, all 107 UI tests, Svelte check, ESLint, and Prettier.
- [x] Pass the clean Compose integration suite across both discovery modes, compatibility replay,
  CRUD, billing, validation, permissions, AD/passive persistence, and daemon lifecycle cleanup.

## Blockers and Decisions

- Authorized real network-device targets and suitable read-only test accounts must be identified
  before the three device-family validation items can complete.
- The remote inventory has no authorized Cisco IOS, Comware, or ArubaOS-Switch lab targets, so
  those hardware/channel/paging checks cannot be completed without targets and read-only accounts.
- No authorized disposable Linux host, real AD/KDC realm, UniFi controller, or approved live passive
  capture window has been supplied. Their production-protocol validations cannot be completed by
  inventing targets, credentials, tickets, or packet-capture authority.
- Do not print or copy live daemon or discovery credentials during remote verification.
- The remote Compose file uses mutable `:forkthat` image tags. Before each deployment, verify the
  tag digest matches the intended immutable revision tag, then pull and recreate deliberately.
- Kerberos support is intentionally container-image-only on Linux: standalone daemon downloads keep
  their existing static musl compatibility contract and do not advertise GSSAPI. The Debian/glibc
  daemon image advertises `active_directory_gssapi`; the server and UI fail closed for every daemon
  without that explicit feature. Enabling it also requires the opt-in read-only single-file ccache
  overlay documented in `docs/AD_KERBEROS_SYSTEM_CCACHE.md`.
- The Kerberos overlay is not enabled on the deployment host. Its read-only mount prevents cache
  mutation but not ticket disclosure if the stock root/privileged/host-networked daemon (which may
  also have the Docker socket) is compromised. Enabling it requires explicit administrator risk
  acceptance, a dedicated short-lived principal, and removal of unnecessary runtime authority.
- `ldap3` constructs a BER entry before Scanopy can apply its 64 KiB normalized-entry check. LDAP
  count/size/deadline bounds remain enforced, and the residual pre-check allocation risk is
  documented with a process/container memory-limit recommendation.

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
- Hotfix commit `0796648bd` was pushed and GitHub Actions run `29634521583` published matching
  linux/amd64 and linux/arm64 server/daemon manifests. Mutable `forkthat` digests matched immutable
  revision tags before deployment; both remote containers are healthy on revision `0796648bd`.
- Remote scan validation reached scanning progress 42% before a separate Compose destroy removed
  server, daemon, and PostgreSQL. The stack was restored with persisted data intact, and exact-name
  temporary credentials, API keys, and validation containers were verified absent (zero rows/items).
- Kerberos feasibility review confirmed that `ldap3` 0.12.1's Unix GSSAPI path is native FFI
  (`cross-krb5` -> `libgssapi` -> `libgssapi-sys`), while the published daemon is static musl and
  its container has neither GSSAPI runtime libraries nor a mounted ccache. A production-valid glibc
  daemon build and strict read-only ccache contract are documented; no fake credential was added.
- The production-valid Kerberos path is now implemented without weakening standalone releases:
  default Windows/macOS/static-musl daemons remain feature-free, while Debian/glibc AMD64/ARM64
  container builds compile `ad-gssapi`, load MIT Kerberos runtime libraries, and advertise an
  explicit persisted capability. WSL feature-enabled check/tests pass, including 23 focused AD
  tests, exact-principal/system-ccache validation, forward-compatible capability negotiation,
  exact host/IP authorization, sanitized failures, and equal-timestamp replay protection.
- UniFi real-loopback HTTPS verification passes 13/13 focused Linux tests, covering trusted and
  untrusted certificates, assigned-IP routing with DNS SNI, proxy bypass prevention, redirect
  refusal, response streaming limits, interface merge safety, and credential redaction.
- Squawk reports zero findings for all three new migrations. Passive observation regressions now
  cover typed/no-raw facts, VLAN/IPv6 boundaries, fragment rejection, in-place fact refresh,
  protocol-expiry filtering, live-correlation cleanup, and a serialized 10,000-row cap.
- Coordinated package version is `0.19.1`. Generated contracts contain 155 full and 104 public API
  paths, 309 schemas, 18 database enums, and 12 credential variants. The generated `v0.19.1`
  compatibility set contains 48 daemon-to-server, 1 DaemonPoll, and 7 ServerPoll exchanges; tracked
  historical fixtures remain byte-for-byte unchanged.
- Final frontend verification passes 16 Vitest files/102 tests, Svelte check with zero
  errors/warnings, ESLint, and Prettier. Final Rust 1.90 library verification passes 592 tests with
  6 ignored; strict server/daemon/`ad-gssapi` Clippy and format checks pass.
- The final post-audit clean Docker integration passed in 890.19 seconds, including
  DaemonPoll/ServerPoll discovery, historical compatibility replay, CRUD/billing/validation/
  permissions, AD replacement and
  100-run retention, passive cross-network rejection/deduplication/expiry/correlation/10,000-row
  retention, and daemon lifecycle cleanup.
- The final-tree linux/amd64 production server (198 MB) and Debian/glibc daemon (192 MB) images
  build successfully from pinned bases. The server contains the built UI and all three new
  migrations; the daemon executes its smoke test and dynamically loads `libgssapi_krb5`, `libkrb5`,
  and `libkrb5support` with no missing libraries. Native linux/arm64 proof remains the publication
  gate.
- Release security review found no blocking application authorization or secret-handling defects.
  Publication now requires Rust/UI verification, locked Cargo advisory audit, critical image CVE
  scans, both image families and architectures, immutable-manifest inspection, and one combined
  mutable-tag promotion. Formal releases capture one post-fixture SHA for every build and source
  SBOM. Production and development base images are pinned to multi-architecture manifest digests;
  `actionlint` v1.7.7 passes both changed workflows.
- The 2026-07-18 exact-lock advisory pass resolved every actionable advisory:
  `crossbeam-epoch 0.9.20` and `quick-xml 0.41.0`/`plist 1.10.0` remove the pointer and XML
  advisories. Replacing the legacy PostHog SDK with a bounded sender on the existing Reqwest 0.12
  stack removes the old Rustls advisories without introducing the forbidden AWS-LC provider.
  `serial_test 3.5.0` removes the informationally unsound `scc 2.4.0` path. `cargo audit` exits
  successfully with only the documented, unfixed RUSTSEC-2023-0071 RSA ignore and six non-denied
  maintenance/yank warnings.
- The reviewed release tree is split into independently revertible commits: `9bf0ede2b` (secure
  dependency baseline), `1c8b26acf` (AD and UniFi), `fe1c5e280` (passive observations),
  `2743df2ce` (contracts and UI), and `b82142665` (multiarch publication hardening).
- Release-ledger commit `883c53d33` and clean-runner repair `a74e423ad` were pushed to `forkThat`.
  The first guarded publication run (`29647400206`) correctly stopped promotion when clean-runner
  fixture generation and ARM64 scan-platform selection failed. Replacement run `29648108145`
  passed all four native image builds, full Rust/UI/source verification, locked advisory audit,
  four critical CVE scans, manifest verification, and atomic mutable-tag promotion.
- GHCR mutable and immutable indexes match exactly. Server index
  `sha256:cec04a689782e03c3f81c3ca988380cd7644e63e022f5fa78b4a8302a4ec983a` contains AMD64
  `sha256:c21ad50edac8d4b56fcaaa701d7ee9eeb7a48ab28690c67aa5012e828be10fc5` and ARM64
  `sha256:f609370014d49bf5904da90bfbd2326a87ea4ff32e440305d41bffbece4cf6a5`.
  Daemon index `sha256:d55b8dc94caaf87fb8e6350cacfd865af6047037b813381736956d138008d955`
  contains AMD64 `sha256:e46553822110a83109e909e07cda738756c2783b5ae2298d2ee7f39026176cf7`
  and ARM64 `sha256:9f9d7c64a3b987b4cb8078541af140a4b940596b436057a0d1c4e8d0ec420c9e`.
- Controlled ARM64 deployment on 2026-07-19 recreated only server and daemon; PostgreSQL remained
  healthy and untouched. Both containers are healthy at revision `a74e423ad`, `/api/version`
  reports `0.19.1`, and migrations `20260718120000`, `20260718121000`, and `20260718122000` are
  recorded successful.
- The external stack recreation had redirected daemon configuration from the intact
  `scanopy_daemon-config` volume to an empty `/opt/scanopy/daemon-config` bind directory. The single
  existing `config.json` was restored without displaying its contents. The daemon then reconnected
  under its persisted identity, reported version `0.19.1` and feature
  `active_directory_gssapi`, and cleared its unreachable state.
- Production safety verification confirms passive collection is unset in the environment and
  resolves to `false`, the Kerberos ccache overlay remains disabled, and the ARM64 daemon links the
  required GSSAPI/Kerberos libraries without missing dependencies. No live scan was started because
  the external operator has not been shown quiescent and no authorized external collector targets
  or passive-capture window were supplied.
- Manual topology layout is stored in the additive `topology_node_positions` table. OpenAPI now has
  157 full paths and 312 schemas; the public contract remains at 104 paths because the mutations are
  internal member-session endpoints. The focused topology suite passes 76/76 tests.
- Local reverse DNS continues to use `dns_lookup::lookup_addr`, so it follows the daemon machine's
  operating-system/NSS resolver configuration. Five focused tests cover normalization, unusable
  addresses, resolver failure, cancellation, timeout, and detached blocking-call concurrency.
- The clean Compose integration suite passes in 681.57 seconds and removes its test containers,
  networks, volumes, and local test images after the run.
- This section is implemented and locally verified on `forkThat`, but is intentionally not yet
  pushed, published, or deployed. The remote stack remains on revision `a74e423ad` / version
  `0.19.1` until an explicit release request authorizes those external changes.

# Scanopy Fork — Session State and Handoff

## Purpose

This document records the work completed in this conversation and the current work in progress so
development can move to the 16-core/128 GB development machine without relying on chat history.
It intentionally contains no passwords, SNMP secrets, API keys, enrolment tokens, or customer
artifacts.

## Repository State

- Repository: `/home/osit/scanopy`
- Branch: `forkThat`
- Baseline HEAD before the handoff commit: `64f23d45e4ddd7d28f4c925e01736a522f59f264`
- Baseline commit: `64f23d45e Fix LLDP discovery on high-index switches`
- The SSH collector work described below is preserved in the handoff commit on `forkThat`, but is
  **not deployed**.
- The handoff commit contains the SSH/provenance changes listed below and was formatted with
  `cargo fmt` before publication.

The reference repository is `/home/osit/codexnet`, branch `main`, HEAD
`d7aec2f7b2a0e4491b5635f303a87087587d644a`. It has one unrelated untracked user file,
`docs/spec-refinement-notes.md`, which must be preserved.

## Work Already Completed and Deployed

Earlier work on `forkThat` removed the Community/MSP artificial network-count restrictions and
restored the UI/API flows needed to add and name networks. GitHub Actions publishes the fork's
server and daemon images to GHCR. The local `/opt/scanopy` deployment was changed to use those fork
images.

LLDP discovery was then repaired for switches whose SNMP table uses large interface indexes. The
fix is commit `64f23d45e`; corresponding multi-architecture images were published and deployed.
The daemon configuration volume was corrected to mount at `/root/.config/daemon`, the daemon was
re-enrolled without changing its intended network, and its credential was persisted. A fresh scan
completed successfully and the user confirmed that L2 topology populated. Do not expose or copy
the live daemon credential while moving development environments.

The currently deployed images represent commit `64f23d45e`, not the SSH work in this document.

## Why New Collectors Are Needed

Scanopy currently has native credentialed integrations for SNMP, Docker, and Podman. Its UI did not
offer SSH or Active Directory credentials, so scans could not authenticate to Linux/Unix hosts,
network-device CLIs, domain controllers, or UniFi controllers. CodexNet already contains tested
reference designs for:

- bounded collector scheduling and target approval;
- SNMP, read-only SSH, Active Directory LDAP/Kerberos, and UniFi collection;
- passive mDNS, DHCP, and neighbor observations;
- opaque credential references, secret redaction, timeouts, cancellation, and partial failure.

The intended Scanopy sequence is SSH first, then Active Directory, UniFi, and passive observations.
Scanopy already has a capable SNMP implementation, so CodexNet SNMP ideas should enhance that
integration rather than create a duplicate collector.

## Licensing and Provenance Decision

CodexNet's `pyproject.toml` currently declares `license = { text = "Proprietary" }`. During this
conversation Robin Collins stated that CodexNet is his codebase/GitHub repository and expressly
authorized relicensing and incorporation into this Scanopy fork. Scanopy is AGPL-3.0.

`docs/CODEXNET_PORTING.md` was added to preserve that authorization and the functional provenance
inside the repository. The port is being written as native Rust integrated with Scanopy rather
than introducing a proprietary Python runtime dependency. Before public release, keep the
provenance file in the commit and, ideally, add a matching durable licence grant or dual-licence
notice to the CodexNet repository. This is a compliance record, not legal advice.

The SSH implementation uses `russh 0.62.2`, licensed Apache-2.0, with default features disabled and
the pure-Rust `ring` and `rsa` features enabled. Apache-2.0 is compatible for inclusion in this
AGPL-3.0 work. Older Russh versions were deliberately not selected because current RustSec
advisories affect versions before the patched line.

## Current SSH Implementation

### Credential model

Two stored credential types have been added:

1. `SshPassword`: username, password, port, platform, host-key policy, and optional daemon-local
   `known_hosts` path.
2. `SshPrivateKey`: username, private key, optional key passphrase, port, platform, host-key policy,
   and optional daemon-local `known_hosts` path.

Both can target the daemon host, selected host IPs, or an entire Scanopy network. Secrets reuse
Scanopy's `SecretValue`/`ResolvableSecret` model, including inline-or-file support, redacted update
merging, daemon-side file resolution, and redacted `Debug` output. Strict host-key verification is
the default and requires an absolute `known_hosts` path. `AcceptUnknown` is available only as an
explicit choice.

The dynamic credential metadata defines all fields required by the generic Svelte credential
wizard. Platforms are explicit: Linux/Unix, Cisco IOS, HP/HPE Comware, and ArubaOS-Switch. There is
no automatic platform guess because sending commands from the wrong family is unsafe and noisy.

### Daemon integration

`backend/src/daemon/discovery/integration/ssh/mod.rs` implements the native integration. It:

- gates probing on the configured TCP port;
- verifies the server host key according to the credential policy;
- authenticates by password or private key without putting secrets in arguments or logs;
- carries the authenticated session from `probe()` into `execute()`;
- runs commands only from static, platform-specific read-only allowlists;
- applies a 20-second per-command timeout, 1 MiB output limit per command, 64 KiB stored system
  description limit, cancellation checks, and a 180-second integration timeout;
- populates Linux hostname/sysName and a bounded sysDescr summary;
- exposes SSH as a credentialed `ClientProbe`, allowing non-standard configured ports to match the
  existing SSH service definition.

The Cisco, Comware, and Aruba command lists were adapted from CodexNet's approved read-only
profiles. The Linux profile currently collects hostname, kernel/OS information, addresses, routes,
and virtualization type.

## Modified and Added Files

Tracked modifications:

- `backend/Cargo.toml`
- `backend/Cargo.lock`
- `backend/src/daemon/discovery/integration/mod.rs`
- `backend/src/server/credentials/impl/mapping.rs`
- `backend/src/server/credentials/impl/types/fields.rs`
- `backend/src/server/credentials/impl/types/metadata.rs`
- `backend/src/server/credentials/impl/types/mod.rs`
- `backend/src/server/services/definitions/ssh.rs`
- `backend/src/server/services/impl/patterns.rs`

New files:

- `backend/src/daemon/discovery/integration/ssh/mod.rs`
- `backend/src/server/credentials/impl/types/ssh.rs`
- `docs/CODEXNET_PORTING.md`
- `SOL-STATE.md`

`Cargo.lock` currently has a large diff (approximately 1,179 additions/134 removals) because modern
Russh brings a current pure-Rust cryptography graph. `base64ct` was changed from the old exact
`1.6.0` pin to the compatible `1.8` line because Russh's `ssh-encoding` requires it. This change
must receive regression and dependency review because `base64ct` is also used by existing auth and
email code.

## Verification Completed

- `cargo check --lib` — **passed** after resolving the `base64ct` constraint.
- `cargo fmt` — **passed/applied** immediately before writing this handoff.
- The selected Russh release and feature set were checked for licence and current security status.

The initial ARM build was very slow: `cargo check --lib` took about 11 minutes. A subsequent
`make generate-fixtures` spent roughly 23 minutes compiling/linking, with the final Scanopy link
using about 4 GB RAM and one core. It was intentionally interrupted when this handoff was requested.
The generator executable had not run, so no metadata fixture or translation output was produced.

## Not Yet Verified or Completed

- `make generate-fixtures` has not completed.
- `make generate-types` has not run; `ui/src/lib/api/schema.d.ts` therefore does not yet contain the
  SSH credential schemas.
- The database enum baseline fixture does not yet list `SshPassword`/`SshPrivateKey`.
- Focused Rust tests, the complete unit suite, Clippy, UI lint/type checks, and multi-architecture
  container builds have not run.
- No mock SSH server integration test exists yet. Current tests only cover allowlist presence,
  UTF-8-safe truncation, and standard/custom port mapping.
- The WIP is preserved by the handoff commit, but no images containing it have been published to
  GHCR or deployed to `/opt/scanopy`.

## Important Design Risks to Resolve

1. **Daemon compatibility floor:** SSH types currently declare minimum daemon version `0.18.0`,
   while the package version is `0.17.3`. The server will correctly refuse to send SSH credentials
   to a `0.17.3` daemon. Before testing through the UI, either perform a coordinated version bump or
   choose an intentional fork-specific next version and update the compatibility floor.
2. **Network-device channel behavior:** the initial implementation uses SSH `exec` channels.
   CodexNet used Netmiko interactive sessions and paging controls. Some Cisco/Comware/Aruba devices
   may reject exec requests or paginate large output. Add mocked protocol tests and an authorized
   lab test before claiming network-device collection works.
3. **Parsing depth:** the collector currently persists hostname and a system-description summary.
   It does not yet normalize CLI interface, VLAN, MAC, ARP, LLDP/CDP, inventory, PoE, or environment
   output into Scanopy entities. SNMP remains the working source of L2 topology at this stage.
4. **Output retention:** non-summary command output is held only in memory and discarded. Decide
   which normalized fields belong in existing Host/Interface/VLAN models before adding persistence;
   do not store unrestricted raw customer CLI output by default.
5. **Dependency size/supply chain:** Russh 0.62.2 is current and patched, but has a large crypto
   dependency graph including release-candidate transitive crates. Run `cargo audit`, inspect
   licences, test musl/ARM64 builds, and assess image-size impact.
6. **Host-key bootstrap UX:** strict mode is safe but requires placing a `known_hosts` file inside
   the daemon container/host and mounting it persistently. Document this and decide whether Scanopy
   should support fingerprint pinning or a deliberate trust-on-first-use workflow.
7. **Service matching regression:** SSH service detection changed from port-only to
   `Port(SSH) OR ClientResponse(SSH)` to support custom ports. Add a regression test for both paths.

## Recommended Resume Procedure

Fetch and check out the pushed `forkThat` branch on the development machine. The handoff commit
contains the formerly dirty working tree, including the new files.

On the development machine:

```bash
cd /path/to/scanopy
git fetch origin
git switch forkThat
git pull --ff-only origin forkThat
git status --short
git branch --show-current
cargo fmt --manifest-path backend/Cargo.toml -- --check
cd backend && cargo check --lib
```

Then address the compatibility version decision and run generation:

```bash
cd /path/to/scanopy
make generate-fixtures
make generate-types
```

Inspect generated changes carefully, especially credential metadata, integration fixtures,
translations, OpenAPI schemas, and `backend/tests/fixtures/db_enum_baseline.json`. Then run focused
checks before the expensive full suite:

```bash
cd backend
cargo test daemon::discovery::integration::ssh
cargo test server::credentials
cargo clippy --lib -- -D warnings
cd ..
make test-unit
make lint
```

Add a mock SSH server test covering strict host-key success/failure, password rejection, private-key
auth, timeout, oversized output, cancellation, and secret-free diagnostics. After that, test Linux
against an explicitly authorized disposable host. Test each network-device family only against an
explicitly authorized lab device and verify that the account cannot enter configuration mode.

Once the vertical slice is complete, commit it as one reviewed logical change, push `forkThat`, let
the GHCR workflow publish both server and daemon images, update `/opt/scanopy`, and verify a scan
without exposing credentials. Only then begin the Active Directory slice.

## Planned Follow-On Slices

### Active Directory

Add separate LDAPS password and Kerberos/system-ccache credential transports associated with the
existing Active Directory/LDAP/Kerberos service definitions. Port only documentation-focused
queries for domain, forest, sites, subnets, domain controllers, trusts, computers, and explicitly
configured group membership. Exclude password material, LAPS attributes, credential dumping,
Kerberoasting, AS-REP roasting, and attack-path collection. A schema decision is required because
Scanopy does not yet model all AD entities.

### UniFi

Add controller URL/site/TLS settings plus password (and, if supported by the selected controller
API, token) credentials. Reuse Scanopy hosts/interfaces/topology where possible. TLS verification
must default on; self-signed exceptions must be endpoint-specific and explicit.

### Passive collectors

mDNS, DHCP, and neighbor observations are not credential types. They need a daemon-level passive
ingestion lifecycle, bounded structured observations, provenance/confidence, retention, and
correlation rules. They should not be forced into the per-host credential integration registry.

## Definition of Done for SSH

The SSH slice is complete only when generated UI/API artifacts expose both credential types,
compatibility/version handling is intentional, secret redaction and host-key verification are
tested, mock and focused tests pass, Linux enrichment works on an authorized test host, supported
network devices have verified channel/paging behavior, musl multi-architecture images build, GHCR
publishes both images, and the deployed fork completes a scan using persisted credentials.

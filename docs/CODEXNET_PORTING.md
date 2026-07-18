# CodexNet Collector Porting

The collector work in this repository may be adapted from the separately developed
`robin-collins/codexnet` project. Robin Collins, the recorded author and copyright holder of
CodexNet, authorized that work to be relicensed and incorporated into this Scanopy fork under
the GNU Affero General Public License, version 3.

Porting should preserve functional provenance in commit messages and module documentation. New
Rust code, tests, and documentation added here are part of the AGPL-3.0-covered Scanopy work; no
CodexNet dependency or proprietary runtime component is required to build or operate it.

The following CodexNet safety properties remain requirements:

- collectors perform documentation-focused, read-only operations only;
- targets must already be in the Scanopy discovery scope;
- credentials are resolved only for their assigned targets and are never logged;
- network calls have bounded timeouts, output sizes, and cancellation;
- one collector failure must not stop unrelated discovery integrations;
- SSH commands use a fixed allowlist, AD collection excludes credential-access and offensive
  enumeration, and TLS/SSH identity verification is explicit.

Initial implementation order is SSH, Active Directory, UniFi, then passive mDNS/DHCP/neighbor
observations. SNMP already has a native Scanopy integration and should receive targeted
enhancements rather than a second collector.

Operational setup, trust policy, persistence boundaries, and validation requirements for the
native SSH slice are documented in [`SSH_DISCOVERY.md`](SSH_DISCOVERY.md).
The security, licence, advisory, and multi-architecture assessment is recorded in
[`SSH_DEPENDENCY_REVIEW.md`](SSH_DEPENDENCY_REVIEW.md).

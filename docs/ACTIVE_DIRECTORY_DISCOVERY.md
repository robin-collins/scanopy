# Active Directory discovery

Scanopy collects a bounded, documentation-only Active Directory inventory through an explicitly
assigned credential and host/IP target. The initial production transport is certificate-verified
LDAPS with a least-privilege simple-bind account.

## Credential and target controls

An LDAPS credential contains a bind DN, password, controller DNS name, TCP port, base DN, optional
private CA bundle, and an optional newline-separated list of documentation group DNs. It can only
be assigned to hosts. The daemon connects TCP to the assigned IP address while using the configured
DNS name for TLS certificate validation, so DNS resolution cannot redirect the connection.

TLS verification is always enabled. A private CA can extend the trust roots for that one credential;
there is no global insecure-certificate switch and no plaintext LDAP fallback. Password and CA file
references are resolved on the daemon with bounded reads and are never written to logs or inventory.

The server re-authorizes every collection submission against the authenticated daemon, active
discovery session, network, stored credential type, and exact credential/host/IP assignment. The
collector identity is derived server-side from the credential and IP rather than accepted from the
daemon.

## Collected inventory

Fixed LDAP filters and attribute allowlists cover:

- domain and forest names and functional level;
- sites and site-linked subnets;
- domain controllers and computer documentation fields;
- trusted domains; and
- only the explicitly configured groups and their direct member object GUIDs.

Stable entity identities use AD `objectGUID` values. Group membership stores only the group and
member GUID relationship, never member names or distinguished names. Trust partners are validated
DNS names and use an opaque derived relationship identifier.

Scanopy never requests or stores passwords, password hashes, LAPS attributes, tickets, keytabs,
roasting data, user-enumeration data, unrestricted LDAP responses, or attack-path information.

## Bounds and replacement semantics

Each operation has connection, search, and total deadlines and observes discovery cancellation.
Streaming searches use server size/time limits, a 64 KiB per-entry cap, fixed per-kind counts, at
most 16 configured groups, and at most 100 direct members per group. A collection request is capped
at 3,000 normalized entities and 8 MiB.

Any missing naming context, missing configured group, malformed/oversized entry, absent required
GUID, or LDAP size/admin limit marks the run partial. Only a successful, non-truncated, newer run
atomically replaces current inventory. Partial, failed, truncated, and stale delayed runs retain
bounded provenance but cannot delete or overwrite current inventory. At most 100 run records are
kept for each credential target.

## Kerberos status

The system-credential-cache transport requires native GSSAPI and a glibc daemon build. Its exact
build and deployment contract is recorded in `AD_KERBEROS_SYSTEM_CCACHE.md`; the credential must not
be exposed until both published daemon architectures can consume a read-only administrator-managed
cache and principal selection is verified.

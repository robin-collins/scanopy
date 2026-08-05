# Active Directory Kerberos/system-ccache transport

## Status

Implemented for Linux daemon builds that advertise the `active_directory_gssapi` capability.
The fork and official container publication paths build native AMD64/ARM64 Debian/glibc daemon
images from source with the `ad-gssapi` feature. Standalone Linux downloads preserve their existing
static musl contract and, like default, macOS, and Windows builds, advertise no GSSAPI capability.
The server fails closed: it neither assigns nor dispatches the Kerberos credential to a daemon that
did not explicitly advertise support, even when the daemon version is otherwise new enough.

The existing password transport remains certificate-verified LDAPS. No password, keytab, ticket
creation, ticket export, Kerberoasting, or AS-REP-roasting behavior is part of this design.

## Native dependency rationale

Scanopy uses `ldap3` 0.12.1 with Rustls. Its `gssapi` feature adds this Unix dependency chain:

```text
ldap3 -> cross-krb5 0.4.2 -> libgssapi 0.9 -> libgssapi-sys 0.3.4
```

This is not a pure-Rust transport on Unix:

- `libgssapi-sys` runs bindgen and therefore needs Clang, libclang, and Kerberos headers while
  compiling.
- Its build script locates MIT `libgssapi_krb5.so` or Heimdal `libgssapi.so` and emits a native
  link directive. It does not vendor a Kerberos implementation.
- The former stock cross-rs musl build images contained no target-built Kerberos development
  library; linking a host glibc GSSAPI library into that target was not safe.
- The production Dockerfile therefore builds on Debian and installs the matching runtime
  libraries. The base Compose file remains cache-free; administrators opt in with the dedicated
  overlay so the server container never receives the cache.

Invoking `ldapsearch`, silently falling back to a simple bind, or accepting a keytab/password would
not satisfy the transport contract and is not used as a workaround.

## Production build implementation

The Linux daemon container is distributed with a dynamically linked Debian/glibc binary for both
AMD64 and ARM64. Standalone downloads remain static musl binaries without this optional feature:

1. Build each architecture in a Debian Bookworm Rust 1.90 stage with `clang`, `libclang-dev`,
   `pkg-config`, and `libkrb5-dev` installed.
2. Enable `ldap3` features `tls-rustls-ring,gssapi` and add the matching `cross-krb5` dependency so
   Scanopy can acquire the specifically configured client principal from the default ccache.
3. Install `libgssapi-krb5-2` and `libkrb5-3` in the daemon runtime image.
4. Linux container artifacts use glibc/GSSAPI. Explicit persisted daemon feature negotiation
   prevents version-only dispatch to standalone, default, or non-Linux builds.
5. Run the multi-architecture image build in native builders where possible. QEMU buildx is a valid
   fallback, but must execute a smoke test that loads the binary and GSSAPI shared libraries.

Changing only the runtime image is insufficient: the daemon binary itself must be built against the
same ABI family and GSSAPI implementation.

## Credential and wire contract

The separate credential variant has only these authentication fields:

- `principal`: required, bounded Kerberos client principal (for example,
  `scanopy-reader@EXAMPLE.COM`).
- `use_system_ccache`: required to be exactly `true`; it is an explicit acknowledgement, not a mode
  selector.
- `server_name`, `port`, `base_dn`, optional CA certificate, and bounded configured group DNs: the
  same endpoint, TLS, and read-scope controls as the LDAPS password transport.

There must be no password, keytab, ccache content, ticket, or ticket-export field in storage or on
the server-to-daemon wire.

On Unix, acquire an initiating `cross_krb5::Cred` for the configured `principal` and pass it to
`ldap3::Ldap::sasl_gssapi_cred_bind`. Do not call the default-principal helper: requiring a principal
prevents an unrelated default ticket in a shared cache from being selected silently. Connect TCP to
the approved target IP, use `server_name` for TLS certificate validation, and use the same
`server_name` for the `ldap/<server_name>` service principal. Keep TLS verification mandatory and
retain the existing read-only filters, entry limits, deadlines, cancellation, and normalized result
handling.

## Runtime ccache contract

The daemon reads an already-issued ticket from the operating environment. Scanopy does not create,
renew, copy, upload, download, or delete tickets.

### Privilege boundary and explicit risk acceptance

The stock Compose daemon currently runs as root with host networking and `privileged: true`; it also
receives the Docker socket unless Docker discovery is disabled. A read-only bind mount prevents
Scanopy from modifying the cache, but it cannot prevent a compromised daemon from reading and
exfiltrating reusable tickets. Do not enable the Kerberos overlay unless the administrator accepts
that trust boundary.

Use a dedicated, least-privileged, short-lived collection principal and cache. Remove the Docker
socket mount when Docker discovery is unnecessary, restrict the host and container with the
smallest capabilities the deployment supports, and apply process/container memory limits. Never
mount a workstation login cache, a directory-admin cache, or a cache directory containing tickets
for other principals.

For a container deployment:

- Mount exactly one administrator-managed cache path read-only, for example
  `/run/scanopy-krb5/ccache`.
- Set `KRB5CCNAME=FILE:/run/scanopy-krb5/ccache` in the daemon container.
- Ensure the cache file is readable by the daemon UID and is not mounted into the server container.
- If a non-default Kerberos configuration is required, mount a read-only `krb5.conf` and set
  `KRB5_CONFIG`; never store its contents in a Scanopy credential.
- Prefer a dedicated daemon identity/cache. A root-owned host cache must not be exposed wholesale.

The provided opt-in Compose overlay enforces the single-file read-only mount:

```sh
SCANOPY_KRB5_CCACHE=/secure/admin/path/ccache \
  docker compose -f docker-compose.yml -f docker-compose.kerberos.yml up -d
```

The daemon accepts only an absolute `FILE:` cache path, confirms that it is readable, and refuses
to use it if the daemon process can open it for writing. Other cache types, missing/expired caches,
and a cache without the configured principal fail without returning GSSAPI details to the API.

LDAP result size, count, and deadline checks are enforced by Scanopy, but `ldap3` constructs a BER
entry before Scanopy can measure its normalized size. Container/process memory limits are therefore
part of the defense against an oversized response from a trusted-but-compromised directory server.

For a systemd deployment, set the same environment variables with an `EnvironmentFile` whose
permissions are managed by the administrator. The service principal recorded in Scanopy must match
the principal available in that cache.

## Verification required before enabling the credential

- AMD64 and ARM64 daemon images compile, load all shared libraries, and start successfully.
- A mock or disposable Kerberos realm proves that the configured principal succeeds and a different
  principal in the same cache is rejected.
- The TCP peer remains the assigned target IP while TLS and the LDAP SPN use the approved DNS name.
- Missing, expired, unreadable, and wrong-principal caches fail with sanitized errors.
- No credential, log, API payload, debug output, or persisted entity contains ticket bytes, keytabs,
  passwords, or raw GSSAPI tokens.
- Existing LDAPS password collection and SNMP/SSH integrations regress cleanly.
- Deployment documentation includes cache rotation: an administrator replaces the external cache;
  Scanopy never mutates it.

## Cache rotation

Issue a replacement cache outside Scanopy for the same configured principal, atomically replace
the administrator-managed file behind the read-only mount, and restart the daemon container if the
container runtime does not expose the replacement inode. Scanopy does not run `kinit`, renew a TGT,
write the cache, export tickets, or delete the old cache.

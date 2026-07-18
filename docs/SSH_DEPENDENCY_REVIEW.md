# SSH Dependency Review

This review records the security, licence, and build assessment for the native SSH integration.
It is an engineering assessment, not legal advice. Re-run the checks whenever Russh or the locked
cryptography graph changes.

## Selected dependency

Scanopy uses `russh 0.62.2` with default features disabled and the `ring` and `rsa` features
enabled. RSA support is retained because RSA host and user keys remain common on network devices.
The selected Russh and `russh-cryptovec 0.62.0` releases are above the `0.60.3` patched floors for
RUSTSEC-2026-0154 and RUSTSEC-2026-0153.

The SSH dependency delta adds 90 registry packages. Their declared licences are permissive
Apache-2.0, MIT, BSD, or compatible combinations. Russh is Apache-2.0 and `base64ct 1.8.3` is
Apache-2.0 OR MIT. No new copyleft dependency conflict was identified. Ten locked transitive
packages are release candidates; they remain a maintenance and supply-chain review point.

## RSA advisory assessment

The exact lock-file scan adds one advisory relative to deployed revision `64f23d45e`:
`rsa 0.10.0-rc.18` is listed by RUSTSEC-2023-0071 (the Marvin RSA timing attack). The deployed
baseline already contains affected `rsa 0.9.10` through existing dependencies, and the advisory
currently has no patched RustCrypto release.

Source review found the SSH integration uses RSA for SSH key conversion and signatures, not the
PKCS#1 v1.5 RSA decryption-oracle operation described by the advisory. RSA support is therefore
retained with this scoped non-reachability assessment. Monitor RustCrypto and Russh releases and
upgrade when a patched compatible line is available. If the threat model changes, the stricter
fallback is to remove Russh's `rsa` feature and explicitly stop accepting RSA host/client keys.

## `base64ct` compatibility

Russh's SSH encoding graph requires the compatible `base64ct 1.8` line instead of Scanopy's old
exact `1.6.0` pin. Existing production consumers are authentication-token generation and Brevo
attachment encoding. Regression tests must preserve these contracts:

- Authentication tokens decode to exactly 32 bytes and use 43-character, unpadded URL-safe
  Base64 without `+`, `/`, or `=`.
- Brevo attachments use canonical padded standard Base64 (`fb ff` encodes as `+/8=`).

The relevant minimum supported Rust version change is compatible with Scanopy's pinned toolchain.

## Multi-architecture evidence

GitHub Actions run `29627966115` successfully built and published the SSH-foundation commit
`a68a69f40` for server linux/amd64 and linux/arm64, plus musl daemon binaries/images for amd64 and
arm64. Relative to `64f23d45e`, compressed image growth was approximately:

- Server amd64: 24,503 bytes (0.05%)
- Server arm64: 21,350 bytes (0.04%)
- Daemon amd64: 1,192,679 bytes (1.98%)
- Daemon arm64: 1,206,572 bytes (2.04%)

Only the daemon binary is musl; the server runtime image is Debian/glibc. A final image build is
still required for the completed `0.18.0` changes. For reproducibility, future work should replace
the mutable Cross `:main` images and cargo-chef tag with reviewed digest pins.

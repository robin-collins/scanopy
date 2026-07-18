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

The 2026-07-18 exact lock-file scan reports only RUSTSEC-2023-0071 (the Marvin RSA timing attack),
for `rsa 0.9.10` in the existing JWT/OIDC graph and `rsa 0.10.0-rc.18` in Russh. The advisory has no
patched RustCrypto release. The project-local Cargo audit configuration ignores only this advisory;
new advisories still fail the publication gate.

Source review found the existing JWT/OIDC paths use RSA verification and the SSH integration uses
RSA key conversion, verification, and client signatures, not RSA decryption. A compromised approved
SSH target could still observe client-signature timing, so this is a risk acceptance rather than a
claim of complete non-reachability. RSA support is retained because older network devices commonly
require it. Use a dedicated read-only discovery key, keep exact target/host-key approval, and prefer
Ed25519 whenever the device supports it. Monitor RustCrypto and Russh and upgrade when a patched
compatible line exists. If this residual risk is unacceptable, remove Russh's `rsa` feature and
explicitly stop accepting RSA host/client keys.

That audit also discovered fixed-version advisories in `crossbeam-epoch`, `quick-xml`, and the old
PostHog `reqwest`/Rustls graph. The lock now uses `crossbeam-epoch 0.9.20` and `quick-xml 0.41.0` via
`plist 1.10.0`. The legacy PostHog SDK dependency was removed; the existing bounded analytics
service now sends the same capture payload with Scanopy's already-reviewed Reqwest 0.12 client
stack, avoiding both the vulnerable old Rustls graph and an unwanted AWS-LC provider. Exact-lock
`cargo audit` then exits successfully with the single documented RSA ignore.

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

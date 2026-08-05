# SSH Discovery

Scanopy's SSH integration runs a fixed, platform-specific set of read-only commands. It does not
accept commands from users, guess a network-device family, enter enable/configuration mode, or
persist unrestricted command output. SSH credentials are resolved by the daemon and must be
scoped to explicitly approved hosts or networks.

## Supported credential transports

- Username and password
- Username and private key, with an optional key passphrase

Both transports support Linux/Unix, Cisco IOS, HP/HPE Comware, and ArubaOS-Switch. Set the
platform explicitly: choosing the wrong command family is noisy and can produce misleading data.

## Host-key verification

`Strict` is the default and required production policy. It verifies the target against an
absolute, daemon-local OpenSSH `known_hosts` file. The file must already contain the approved
host key; Scanopy never changes it during a scan.

`AcceptUnknown` disables host-key verification. Use it only for an explicitly trusted,
time-bounded bootstrap or disposable test environment. Replace the credential with `Strict` as
soon as the expected key is known.

The initial SSH slice deliberately does not implement trust-on-first-use (TOFU). Silent TOFU
would turn the first connection into an unaudited trust decision, while an audited TOFU workflow
would require new storage, confirmation, rotation, and concurrency behavior. Fingerprint pinning
can be considered later; a managed `known_hosts` entry already provides explicit key pinning for
this slice.

## Docker Compose bootstrap

Store the `known_hosts` file inside a volume that is already mounted persistently into the daemon,
or add a dedicated read-only bind mount. The path entered in the credential is the path inside the
daemon container, not a host path.

For the `forkThat` deployment at `/opt/scanopy`, the daemon configuration volume is mounted at
`/root/.config/daemon`. A suitable container path is therefore:

```text
/root/.config/daemon/known_hosts
```

Populate the file from a trusted administrative machine or console. Verify the fingerprint
through an independent trusted channel before installing it. Do not use an unauthenticated
`ssh-keyscan` result as the sole basis for trust.

For a dedicated bind mount, add a Compose volume similar to:

```yaml
services:
  daemon:
    volumes:
      - ./daemon-known-hosts:/etc/scanopy/ssh/known_hosts:ro
```

Then configure `/etc/scanopy/ssh/known_hosts` in the SSH credential. Keep the host file readable
only by the deployment administrator and daemon runtime. Compose changes require recreating the
daemon container; confirm that its existing enrolment/configuration volume is unchanged before
doing so.

Non-standard SSH ports use OpenSSH's bracketed host syntax in `known_hosts`, for example
`[192.0.2.10]:2222`. Standard port 22 uses the normal host form.

## Collection and retention boundary

Each command has a 20-second timeout and a 1 MiB output limit. The full integration has a
180-second timeout and checks for scan cancellation. Authentication material is never placed in
process arguments or logs.

The initial persistence contract is intentionally narrow:

- Linux `hostname` can populate the host name and `sysName` fallback.
- Bounded OS/kernel or device-version output can populate a maximum 64 KiB `sysDescr` summary.
- Other command output is held only long enough to run the integration and is then discarded.
- Raw customer CLI output is not stored as an artifact or database field.

Interface, VLAN, MAC, ARP, LLDP/CDP, inventory, PoE, and environmental normalization is deferred
until Scanopy has an explicit model and field-level provenance/confidence policy for those values.
SNMP remains the authoritative implemented source for normalized L2 topology in this slice.

## Validation requirements

Before enabling SSH collection in a production deployment:

1. Confirm strict host-key success and mismatch rejection with the exact deployed file.
2. Test the credential against an authorized disposable Linux host.
3. Test each selected network-device family against authorized lab equipment.
4. Confirm that the account cannot enter enable/configuration mode or write configuration.
5. Confirm paging and SSH channel behavior for the device OS/version.
6. Run a normal SNMP/LLDP scan and confirm existing topology behavior is unchanged.

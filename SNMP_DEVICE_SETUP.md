# SNMP + LLDP device setup for Scanopy

Target state for every device Scanopy should place on the L2 Physical view.
Written for an agent doing the configuration. Scanopy daemon (the only SNMP
manager) is `10.10.10.4`. Network `10.10.10.0/24`. See `SNMP_HOSTS.md` for the
device list and `SNMP_HOSTS_CREDS.md` (gitignored) for secrets and host IDs.

## What Scanopy needs from each device

| Data | MIB | Used for |
|---|---|---|
| sysDescr / sysName / sysObjectID | SNMPv2-MIB `.1.3.6.1.2.1.1` | Host identity, naming |
| Interface table with MACs | IF-MIB `.1.3.6.1.2.1.2`, `.1.3.6.1.2.1.31` | Matching LLDP neighbours by MAC. Without this a device cannot be placed |
| Own chassis id + neighbours | LLDP-MIB `.1.0.8802.1.1.2` | L2 links. Needs an LLDP agent (`lldpd`) with its SNMP sub-agent enabled |
| ARP / IP tables | `.1.3.6.1.2.1.4` | Optional, extra L3 evidence |

Rule of thumb: the SNMP view must expose the whole `.1` tree read-only, not
just the system group. A device that answers `sysDescr` but nothing else shows
up in Scanopy as `SnmpCollectedNothing` and gets no interfaces.

Verify from the daemon host (`osit@10.10.10.4`, net-snmp tools installed):

```bash
snmpwalk -v2c -c <community> <ip> ifDescr        # interfaces present
snmpwalk -v2c -c <community> <ip> ifPhysAddress  # MACs present
snmpwalk -v2c -c <community> <ip> .1.0.8802.1.1.2.1.3.2  # lldpLocChassisId
snmpwalk -v2c -c <community> <ip> .1.0.8802.1.1.2.1.4.1  # lldpRemTable
```

All four must return rows. Use `-v3 -u scanopy -l authPriv -a SHA -A ... -x AES -X ...` for OPNsense.

## Linux (Debian / Ubuntu): krusty 10.10.10.250, ralph 10.10.10.225, any other

Both currently run snmpd with Debian's default `-V systemonly` view, which is
why Scanopy collects nothing beyond sysDescr. Neither runs `lldpd`.

```bash
apt-get install -y snmpd lldpd snmp
```

`/etc/snmp/snmpd.conf` (replace the file; pick a unique community per host,
do not keep `public`):

```
agentaddress udp:161
sysLocation  homelab
sysContact   tech
# Read-only, whole tree, only from the Scanopy daemon
rocommunity  <UNIQUE_COMMUNITY> 10.10.10.4
# Let lldpd publish LLDP-MIB through snmpd
master agentx
agentxsocket /var/agentx/master
```

`/etc/default/lldpd`:

```
DAEMON_ARGS="-x"
```

`-x` enables the AgentX sub-agent so snmpd serves LLDP-MIB. Optional
`/etc/lldpd.d/scanopy.conf` to advertise the FQDN and management IP:

```
configure system hostname krusty.homelab.teamcollins.net
configure system ip management pattern 10.10.10.*
```

Then:

```bash
systemctl enable --now snmpd lldpd
systemctl restart lldpd snmpd
lldpcli show chassis        # chassis id present
lldpcli show neighbors      # switch visible within ~60s
```

Proxmox (krusty) notes: it is on switch ports 1/0/3 to 1/0/6 as a bond. `lldpd`
transmits on the physical members automatically; nothing extra needed. Its
NIC MACs are `a0:36:9f:a8:89:ee/ef`, `38:2c:4a:71:42:8b`, `38:2c:4a:71:44:f6`
and must appear in `ifPhysAddress` for the links to resolve.

Docker host (ralph): the switch reports no LLDP from it today, so `lldpd` is
required, not optional. Docker bridge interfaces will appear in ifTable; that
is fine.

## FreeBSD (OPNsense) 10.10.10.1

Already working: `os-net-snmp` plugin, SNMPv3 user `scanopy`, authPriv,
SHA-1 auth, AES-128 priv, and Scanopy already receives its chassis id and its
LLDP neighbour on `igc1` (switch port 1/0/1). Only verify, do not rebuild:

1. System > Firmware > Plugins: `os-net-snmp` and `os-lldpd` installed.
2. Services > Net-SNMP > General: enabled, listen on LAN (`10.10.10.1:161`),
   SNMPv3 only, community strings empty.
3. Services > LLDPd: enabled, "Enable SNMP subagent" checked, LAN interfaces
   selected (WAN off).
4. Firewall > Rules > LAN: allow UDP 161 from `10.10.10.4` (the default LAN
   allow-any rule already covers this).
5. Verify with the SNMPv3 walk above; `lldpRemTable` should list the switch.

If the view was ever restricted, `/usr/local/share/snmp/snmpd.conf` must read
`rouser scanopy authpriv` with no `-V` view argument.

## Windows 11: MAXPOWER 10.10.10.10, HOMER 10.10.10.11

Both already answer SNMP (Scanopy has their interface tables). Confirm and
harden; run in an elevated PowerShell:

```powershell
# SNMP service (already present on both; idempotent)
Add-WindowsCapability -Online -Name 'SNMP.Client~~~~0.0.1.0'
Set-Service SNMP -StartupType Automatic

$p = 'HKLM:\SYSTEM\CurrentControlSet\Services\SNMP\Parameters'
# Read-only community, unique per host (4 = READ ONLY)
New-ItemProperty -Path "$p\ValidCommunities" -Name '<UNIQUE_COMMUNITY>' -Value 4 -PropertyType DWord -Force
# Only the Scanopy daemon may query; keep localhost as entry 1
New-ItemProperty -Path "$p\PermittedManagers" -Name '2' -Value '10.10.10.4' -PropertyType String -Force
# Advertise the full MIB-II set (bit 0x4F: physical, datalink, internet, end-to-end, applications)
Set-ItemProperty -Path "$p\RFC1156Agent" -Name 'sysServices' -Value 79
Restart-Service SNMP

# Firewall: allow UDP 161 from the daemon only
Get-NetFirewallRule -DisplayGroup 'SNMP Service' | Enable-NetFirewallRule
Get-NetFirewallRule -DisplayGroup 'SNMP Service' | Set-NetFirewallRule -RemoteAddress 10.10.10.4
```

Both hosts already have a unique read-only community configured (values in
`SNMP_HOSTS_CREDS.md`). Reuse them rather than creating new ones.

LLDP on Windows: both machines already transmit LLDP (the switch sees them on
ports 1/0/10 and 1/0/11) using the built-in Microsoft LLDP driver, which
advertises the bare hostname as a locally assigned chassis id, the NIC MAC as
port id, and no sysName. Windows exposes no LLDP-MIB over SNMP. Scanopy's
current resolver cannot match that advertisement; a fork change to match on
the port id MAC is planned. Nothing to configure on the Windows side beyond
keeping the LLDP driver bound to the physical NIC:

```powershell
Get-NetAdapterBinding -ComponentID ms_lldp | Where-Object Name -match 'I211|Realtek'
Enable-NetAdapterBinding -Name '<physical adapter name>' -ComponentID ms_lldp
```

## Scanopy side, after devices are configured

1. Credentials (Platform > Credentials, or `PUT /api/v1/credentials/{id}`):
   update the community string for every host whose community changed.
   Keep one SNMP credential per host, host-scoped to the **live** host record.
2. Remove stale duplicate hosts before rescanning, otherwise MAC and IP
   lookups are ambiguous and links stay unresolved. Known duplicates as of
   2026-09-01: `krusty` `c6c21d9d-706e-4600-a104-0fe7c17c423e` (last seen
   2026-08-07; `ProxmoxSNMP` is bound to this one, rebind it to
   `8438eb00-ba36-4826-b197-239e9f34a0e9`) and `MAXPOWER`
   `b115db62-ac05-46ab-9f18-991fa7e0af06` (last seen 2026-08-30).
3. Trigger a discovery and read the run's warnings:
   `GET /api/v1/discovery?limit=1` then
   `.data[0].run_type.results.warnings`. Success looks like no
   `SnmpCollectedNothing` for any Linux host and no `LldpNeighbourNotFound`
   for `krusty`.
4. Check links: `GET /api/v1/if-entries?network_id=<id>&limit=2000` and count
   entries with a non-null `neighbor`. Expect switch ports 1, 3 to 6, 10, 11
   and 15 to resolve once Linux is fixed and the Windows resolver change lands.

## Known limits that no device setup fixes

- TL-SG2218 exposes LLDP-MIB but no BRIDGE-MIB, so devices without LLDP
  (printer, wireless clients behind the AX11000) cannot be placed by MAC
  table. Its VLAN, ARP and LLDP local-port walks also return nothing; this
  does not affect link resolution.
  - **Investigated 2026-09-02. Standard BRIDGE-MIB/VLAN-MIB/ARP genuinely
    absent, but TP-Link's private FDB MIB works — a real fix is possible.**
    `show running-config` revealed `access-list 900`, bound to all 18
    ports, permitting UDP/161 to the switch only from `10.10.10.4`
    (Scanopy's daemon) and dropping every other source silently. An initial
    test of TP-Link's private FDB MIB (`docs/privateMibs/tplink-l2Bridge.mib`,
    `tpl2BridgeManageDynAddrCtrlTable` at `.1.3.6.1.4.1.11863.6.10.1.2.2`)
    from a non-permitted source timed out and was wrongly read as "not
    implemented" — re-running the identical `snmpwalk`/`snmpget` **from
    10.10.10.4 itself** succeeds. Since Scanopy's daemon *is* `10.10.10.4`,
    this data is genuinely reachable today, just not collected. The
    standard-MIB gaps (BRIDGE-MIB, Q-BRIDGE-MIB/VLAN-MIB, ARP table, CDP,
    LLDP local-ports) are unaffected by this correction — those come from
    Scanopy's own discovery warnings, which already run from the permitted
    `10.10.10.4`, so they're genuinely unimplemented in this firmware
    (`TL-SG2218 1.0`, software `1.1.8 Build 20230602 Rel.73473`). Full
    detail, exact OIDs, and a firmware-upgrade lead (unverified) in
    `TL-SG2218.md`.
- The AX11000 has no SNMP; wireless clients will only ever appear at L3.
- Windows `SnmpWalkDesynchronised` warnings for CDP and VLAN names are noise.

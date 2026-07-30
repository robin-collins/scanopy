# SNMP Host/Application/Topology Data Hydration

## Purpose

This document surveys what Scanopy's SNMP integration currently pulls in, what it collects but
throws away, and what standard SNMP MIBs could add if we wired them up — organized by the entity
each item would enhance (Host, Interface/topology, VLAN, Service/application). It's grounded in
two things:

1. A full read of `backend/src/daemon/discovery/integration/snmp/{mod,oids,queries,types,values}.rs`
   and `backend/src/server/snmp/resolution/resolver.rs`, plus the `HostBase`/`InterfaceBase`/
   `VlanBase`/`ServiceBase` entity structs.
2. Live SNMP capability checks against four real devices already onboarded: an OPNsense router,
   a TP-Link TL-SG2218 switch, a Proxmox VE host ("krusty"), and a Debian VM ("ralph") — see
   [Live fleet snapshot](#live-fleet-snapshot).

Nothing here requires a new collector type. It's all extensions to the existing SNMP integration:
new OIDs to query, existing query results to stop discarding, and one cheap operational change
(installing `lldpd` on the Linux hosts) that pays off immediately with zero new code.

## Current state: what's fully wired today

These fields are collected via SNMP and land on an entity end-to-end, right now, no gaps:

**`Host`** (`server/hosts/impl/base.rs`) — `sys_descr`, `sys_object_id`, `sys_location`,
`sys_contact`, `sys_name` (also used as a hostname fallback when DNS didn't resolve one),
`chassis_id` (from LLDP local chassis), `manufacturer`/`model`/`serial_number` (ENTITY-MIB,
collapsed to the single "best" chassis-class row).

**`Interface`** (`server/interfaces/impl/base.rs`) — `if_index`, `if_descr`, `if_name`,
`if_alias`, `if_type`, `speed_bps` (ifSpeed, overridden by ifHighSpeed×1e6 when present),
`admin_status`, `oper_status`, `mac_address`, six raw `lldp_*` fields, four raw `cdp_*` fields,
`fdb_macs` (single-MAC ports only), `native_vlan_id` / `vlan_ids` (resolved from Q-BRIDGE PVID and
egress-port bitmaps against a per-scan VLAN map).

**`IPAddress`** — `mac_address` enrichment via ipAddrTable↔ifTable join; ARP entries pointing at
*new* remote subnets create whole new `IPAddress`/`Subnet`/`Host` rows (same-subnet ARP entries
are deliberately skipped — they're covered by the direct scan already).

**`Vlan`** — `vlan_number`, `name` from Q-BRIDGE (preferred) or legacy dot1d VLAN tables.

**Topology (`Interface.neighbor`)** — `server/snmp/resolution/resolver.rs` turns raw LLDP/CDP data
into a resolved `Neighbor::Interface(id)` or `Neighbor::Host(id)`, trying in order: remote MAC
against `IPAddress.mac_address` then `Interface.mac_address`; LLDP management address against
`IPAddress.ip_address`; remote port-ID string against `Interface.if_descr`; remote chassis ID
against `Host.chassis_id`; and for CDP, remote device ID against `Host.sys_name`. There's even a
MikroTik-specific quirk that retries bridged port-IDs like `bridge-LAN/ether4-Center` against the
suffix after the last `/`. `Interface.neighbor` is what actually drives topology-staleness
invalidation, so this is the field that puts a line on the topology graph.

## Live fleet snapshot

Verified by direct `snmpget`/`snmpwalk` against the four devices we've onboarded, using the
credentials in `/home/osit/.config/snmp/credentials.env`:

| | OPNsense router | TP-Link TL-SG2218 | krusty (Proxmox) | ralph (Debian VM) |
|---|---|---|---|---|
| sysDescr | `FreeBSD OPNsense... 14.3-RELEASE...` | `JetStream 16-Port Gigabit Smart Switch...` | Linux kernel string | Linux kernel string |
| ifNumber | 8 | 19 | 18 | **126** (mostly Docker `br-*`/`veth*`) |
| ARP (ipNetToMedia) entries | **25** (whole-LAN cache — it's the router) | 6 (mgmt-IP only) | 35 (local only) | 29 (local only) |
| Bridge FDB (dot1dTpFdb) | 0 | **22** | 0 | 0 |
| LLDP local chassis | not present | **present**, 4 remote neighbors | not present | not present |
| Q-BRIDGE VLANs configured | — | 1 (default only) | — | — |
| entPhysicalTable rows | 0 | 0 | 0 | 0 |

Two things fall out of this table that matter for prioritization below:

- **OPNsense's value-add is its ARP table, not topology.** It sees every active MAC on the LAN
  because it's the default gateway. TP-Link's value-add is the opposite: real L2 topology (FDB +
  LLDP) but a tiny, host-local ARP view.
- **Neither Linux host emits LLDP today, but both *could* with zero Scanopy changes.** The daemon
  already queries and fully resolves LLDP data (see above) — `lldpd` just isn't installed on
  either machine yet. Installing it is the highest-value, lowest-effort item in this document.
- **ENTITY-MIB is empty on all four current devices.** The fine-grained per-component collection
  described below (PSUs, fans, SFPs) has zero payoff on this fleet today — it's future-proofing
  for real modular switch/router gear, not something to prioritize now.

## Gaps: collected but discarded (cheap wins, no new OIDs)

The daemon already puts these on the wire and parses them; they just don't reach an entity.

1. **`ifMtu`** — walked in `queries.rs` (`walk_if_table`) and stored on `IfTableEntry`, but
   `InterfaceBase` has no `mtu` field and `convert_snmp_if_entry` never reads it. Adding
   `mtu: Option<i32>` to `InterfaceBase` and one line in the converter captures this for free on
   every device already being scanned. Useful for spotting jumbo-frame mismatches across a link.

2. **`entPhysicalDescr`** — computed into `DeviceInventory.description` in `query_entity_physical`,
   but `mod.rs` only reads `.manufacturer`/`.model`/`.serial_number` off that struct.
   `HostBase.description` is a free-text field already used for other purposes, so this probably
   wants its own column (e.g. `hw_description`) rather than overwriting user-editable text — but
   the data is sitting there unused either way. (Zero payoff on the current fleet — see snapshot
   above — but real payoff the moment a modular switch/router with populated ENTITY-MIB is added.)

3. **Individual ENTITY-MIB component rows** — `query_entity_physical` walks the *entire*
   `entPhysicalTable` (chassis, module, stack, port, powerSupply, fan, sensor — every
   `entPhysicalClass` value) into a map, then collapses it to one "best" row and discards
   everything else. On hardware that populates this table (enterprise switches, chassis routers,
   some UPS/PDU gear), the discarded rows are the actual field-replaceable-unit inventory: PSU
   model/serial, fan status, per-slot module identity, SFP/transceiver inventory. This is a bigger
   lift than the other two items — it needs a new child entity (something like
   `HostComponent { class, description, mfg_name, model_name, serial_number, parent_index }`)
   rather than a field on `Host` — but the raw data collection is already done.

## Gaps: structurally absent from the current OID set

These aren't wiring gaps — `oids.rs` doesn't define these OID groups at all, so this is new
integration surface, not "finish what's there."

### System group

- **`sysServices.0`** (`.1.3.6.1.2.1.1.7.0`) — the OID constant exists in `oids.rs` but is never
  queried anywhere. It's a bitmask of OSI layers the device claims to operate at (physical,
  datalink, network, ...transport, application). Trivial to add to `query_system_info` and would
  give a free, standards-based signal for device classification (router vs. host vs. repeater)
  independent of sysObjectID/sysDescr string matching.

### IP layer

- **`ipAddressTable`** (`.1.3.6.1.2.1.4.34`, IP-MIB, address-family-agnostic) — defined in
  `oids.rs` but `query_ip_addr_table` only walks the legacy IPv4-only `ipAddrTable`
  (`.1.3.6.1.2.1.4.20`). **There is currently no IPv6 address collection via SNMP at all.** Given
  OPNsense and the switch both plausibly run dual-stack, this is a real coverage gap, not just a
  nice-to-have — worth prioritizing above the ENTITY-MIB component work.

### Application / service discovery (entirely new OID groups)

Scanopy's `Service` entity is driven entirely by port-scanning and banner detection today —
`grep` across `services/impl/base.rs` confirms zero SNMP references. None of the following are
touched by any OID currently in `oids.rs`:

- **HOST-RESOURCES-MIB** (`.1.3.6.1.2.1.25`) — the standards-based option, works on any RFC-2790
  compliant agent (which includes stock net-snmp on both Linux hosts we just onboarded):
  - `hrSWRunTable` — running processes: name, path, parameters, run type (operating
    system/independent/etc.), status. This is the closest SNMP equivalent to "what's running on
    this box" without a port scan, and it sees things a port scan can't (local-only services with
    no listening socket, or processes bound to loopback).
  - `hrSWInstalledTable` — installed software inventory (name, ID, install date). Valuable for
    asset/vulnerability tracking independent of what's currently running.
  - `hrStorageTable` — disk/memory utilization (type, size, used). Host health data, not topology,
    but a natural fit for a "Host" detail view.
  - `hrProcessorTable`, `hrDeviceTable` — CPU load and device (network/disk/etc.) inventory.
  - Caveat confirmed against the live fleet: this MIB lives under `.1.3.6.1.2.1.25`, which is
    already inside the `.1.3.6.1.2.1` view we granted `ralph`/`krusty` — no daemon-side
    reconfiguration needed on hosts already onboarded, only new query/parse code.

- **UCD-SNMP-MIB** (`.1.3.6.1.4.1.2021`, net-snmp's own extension) — load average
  (`laLoadTable`), memory (`memory`), disk (`dskTable`), and critically `extendTable` /
  `nsExtendObjects` (the output of `extend`/`exec` directives in `snmpd.conf`) — a way to surface
  arbitrary local script output (e.g. a systemd unit health check, a package-version probe)
  through SNMP without a custom agent. This is Linux/net-snmp-specific (won't help on the TP-Link
  switch or most enterprise gear), so it'd be additive to HOST-RESOURCES-MIB, not a replacement.
  It's outside the current view scope (`.1.3.6.1.4.1`, not `.1.3.6.1.2.1`) — using it on
  `ralph`/`krusty` would need a view change, not just new query code.

- **TCP-MIB / UDP-MIB** (`tcpConnectionTable`, `udpTable`) — listening/established connections by
  local address, local port, remote address, remote port, process ID (on agents that support
  `tcpConnectionProcess`). This is a legitimate SNMP-only alternative to port-scanning for
  discovering what's listening, and unlike a port scan it works through host firewalls that block
  inbound probes but still allow the SNMP query. Lower priority than HOST-RESOURCES-MIB — it tells
  you a port is open, not what's running behind it, so it complements rather than replaces
  banner-based `Service` detection.

None of the above map cleanly onto the existing `ServiceBase` struct (`service_definition`,
`bindings`, `virtualization`, `source`, `tags`, `position` — all currently port-scan-shaped). This
would need either a new "SNMP-observed process/software" child entity distinct from `Service`, or
a `source: EntitySource::Snmp` variant reconciled against existing port-scan-derived services by
port number where `hrSWRunTable`/TCP-MIB overlaps with an existing binding. Worth a design pass
before implementation — flagging the option, not prescribing the shape.

## Gaps: data already on `Host`/`Interface` that's never pattern-matched

`Host.sys_object_id` (vendor enterprise OID) and `Host.sys_descr` are stored as opaque strings and
never interpreted. `HostVirtualization` (`hosts/impl/virtualization.rs`) currently only has
`Proxmox`/`VCenter`/`ESXi` variants, each tied to an *API credential* — SNMP has no path into
virtualization detection at all today. But the raw signal is frequently already sitting in
`sys_descr`: a VM's `sysDescr` commonly contains strings like `QEMU Virtual CPU` or references a
hypervisor's enterprise OID under `sysObjectID` (VMware, Xen, Hyper-V, KVM/QEMU all have assigned
enterprise numbers, and `server/snmp/resolution/generated/enterprise_numbers.rs` already exists in
this codebase for OID→vendor resolution). A lightweight pattern-match pass over `sys_descr`/
`sys_object_id` at ingest time could set a lower-confidence virtualization signal (distinct from
the credentialed-API kind, which should stay authoritative when both are present) purely from data
already being collected today. This wouldn't need any new OID — just new logic in `mod.rs` where
`sys_descr`/`sys_object_id` are already being written to `HostBase`.

## What VLAN reporting is missing

`Vlan.description` exists on `VlanBase` but nothing feeds it — Q-BRIDGE-MIB's
`dot1qVlanStaticName` is collected (that's `Vlan.name`), but there's no OID query for a separate
VLAN description string in the current integration, and most switch agents don't expose one
distinct from the name anyway. Not flagging this as an action item — noting it so it isn't assumed
to be an oversight if someone goes looking for a `Vlan.description` source later; TP-Link's own
`dot1qVlanStaticName` walk currently returns exactly one row (the default VLAN) on our live switch,
so there's nothing to enrich there yet regardless.

## Recommendations, roughly by effort/value

1. **Install `lldpd` on `ralph` and `krusty`.** Zero code changes — the resolver, the OIDs, the
   view scope (`.1.0.8802.1.1.2` is already granted) are all in place. This is the only item in
   this document that's an infrastructure change rather than a Scanopy code change, and it's the
   only way either Linux host gets a real position in the topology graph (right now they have no
   LLDP data, so nothing links them to a switch port).
2. **Wire up `ifMtu` → `Interface.mtu`.** Already collected, one struct field + one converter line.
3. **Add `ipAddressTable` (IPv6) alongside the existing IPv4-only `ipAddrTable` walk.** Real
   coverage gap on dual-stack devices we already scan (OPNsense, the switch), not just a
   nice-to-have.
4. **Add `sysServices.0` to `query_system_info`.** Trivial, standards-based device-role signal.
5. **HOST-RESOURCES-MIB (`hrSWRunTable`, `hrStorageTable`, `hrProcessorTable`) on Linux hosts.**
   Highest ceiling of anything here — genuine application/process visibility distinct from port
   scanning — but needs the new-entity design work called out above before implementation.
6. **ENTITY-MIB per-component inventory as a child entity.** Data collection already happens;
   needs a schema decision (new `HostComponent`-style entity). Deprioritize relative to the above
   — currently zero payoff on the actual fleet since none of our four onboarded devices populate
   `entPhysicalTable` at all; revisit when a modular switch/router with real ENTITY-MIB support is
   added.
7. **`sys_descr`/`sys_object_id` virtualization pattern-matching.** Cheap, additive, no new OIDs —
   but should land as a *lower-confidence* signal that defers to the existing credentialed-API
   virtualization detection when both are present, to avoid regressing an already-reliable source.
8. **UCD-SNMP-MIB `extendTable` and TCP/UDP-MIB listener tables.** Real but narrower value
   (net-snmp/Linux-only for UCD-SNMP; complements rather than replaces port-scanning for TCP/UDP-
   MIB). Lowest priority of the structurally-new items above.

# Passive observations

Passive observation is disabled by default. Set
`SCANOPY_PASSIVE_COLLECTION_ENABLED=true` (or pass
`--passive-collection-enabled=true`) on an authorized `DaemonPoll` daemon to
continuously normalize visible mDNS, DHCP, ARP, and Linux neighbor-table
evidence independently of scheduled discovery and credentials. `ServerPoll`
retains its no-outbound-connections contract and does not start this service.

The capture adapter opens non-promiscuous datalink channels on configured,
active interfaces and recognizes bounded single/double VLAN encapsulation,
ARP, unfragmented IPv4 UDP, and direct-header IPv6 UDP. IPv4 fragments and IPv6
extension/fragment chains are ignored rather than reassembled. It needs the same packet-capture permission already used by
ARP discovery (normally `CAP_NET_RAW` on Linux), not full container privilege.
Failure to open one interface is isolated and does not stop discovery or the
remaining passive collectors. The Linux neighbor-table fallback reads
`/proc/net/arp` without elevated permissions.

Packet bytes are transient. Parsers enforce protocol-specific size and count
bounds and emit only typed mDNS service, DHCP lease, or neighbor mapping facts.
There is no raw-payload field in the wire or database schema. mDNS TXT values
are discarded; only bounded keys are retained.

The daemon queue, duplicate window, retry batch, and server request body are
all bounded. The server authenticates the daemon, derives its network scope,
cross-checks each source against its fact variant and confidence policy, hashes
typed correlation identifiers, and never treats a hostname or leased IP alone
as device identity. Detailed observations and inactive correlation summaries
expire after 30 days; shorter protocol expiry such as mDNS TTL or DHCP lease
time is honored. Identical current facts are refreshed in place, kernel-neighbor
polling is intentionally low-frequency, expired correlations are removed when
no live fact remains, and each daemon/network is capped at 10,000 observation
rows even during high-churn broadcast storms. An hourly server-owned retention
sweep enforces the same rules when collectors are disabled, offline, or removed.

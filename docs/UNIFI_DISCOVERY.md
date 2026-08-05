# UniFi controller discovery

Scanopy collects bounded device and interface inventory from an explicitly assigned UniFi
controller. The collector supports the local-controller session APIs exposed by UniFi OS and
legacy UniFi Network controllers. Use a dedicated, least-privilege local account that can read the
configured site and does not require MFA.

## Authentication scope

The credential contains the controller HTTPS origin, TLS server name, site identifier, modern or
legacy API selection, username, and password. The password may be stored inline or read from a
bounded daemon-local secret file. It is redacted from API responses and diagnostic output.

Token authentication is intentionally not exposed. The selected controller APIs provide a
documented username/password session flow, but this collector has no supported read-only token API
with equivalent modern and legacy behavior. Adding a generic token field would invite operators to
paste browser cookies, CSRF values, or cloud credentials whose scope and rotation Scanopy cannot
enforce. A token transport should be added only alongside a controller API with a documented
read-only token contract and its own request-boundary tests.

The collector rejects accounts that require MFA instead of attempting to bypass or automate the
second factor. It performs only the selected login request followed by the site-scoped device-list
request. Its request allowlist rejects controller writes and all other paths.

## Endpoint and TLS controls

TLS certificate verification is enabled by default. `AllowInvalidCertificate` is an explicit
exception on one credential and affects only that controller client; there is no global TLS bypass.
The configured controller URL must be an HTTPS origin with no embedded credentials, path, query, or
fragment, and its host must exactly match the configured TLS server name.

The daemon connects to the server-approved host IP while retaining the configured DNS name for TLS
SNI and certificate verification. This prevents DNS resolution from redirecting collection to a
different endpoint. Environment HTTP proxies are disabled for these requests because proxy-side
resolution would bypass the approved-IP boundary. Redirect following is disabled, and a redirect
response is treated as a controller failure rather than a new request target.

Requests have connection and total deadlines. Response bodies are streamed into a 4 MiB bounded
buffer. A collection accepts at most 512 devices and 256 ports per device. Endpoint, authentication,
file-resolution, and parsing errors are reduced to secret-free categories before they reach normal
logs or API responses.

## Inventory and merge behavior

For devices within approved discovery subnets, Scanopy can populate normalized hosts, IP addresses,
manufacturer, model, serial number, software description, chassis MAC, management URL, and bounded
port-table interfaces. Devices outside those subnets are ignored.

UniFi port tables are useful enrichment but are not treated as an authoritative SNMP `ifTable`.
When SNMP already supplied interfaces, controller data enriches only an exact interface-index match;
if both sources provide an interface name, the names must also match case-insensitively. Each
controller interface may enrich at most one existing interface. SNMP descriptions and completeness
remain authoritative, so UniFi data cannot prune or replace SNMP interfaces. If only controller
interfaces are available, the set is marked partial.

The selected device response does not provide enough stable, cross-version port identity to create
safe topology edges from uplink hints. Scanopy therefore does not invent uplink interfaces,
neighbors, or links and does not repeat a chassis MAC as a per-port MAC. Topology continues to come
from authoritative sources such as SNMP LLDP/CDP until a separately tested UniFi topology mapping
can identify both endpoints and ports without guesswork.

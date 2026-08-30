# Properties Panel Catalogue

This document is an exact baseline of every Properties/Inspector panel currently implemented in the Scanopy topology UI, as of commit `5d4941ded` on branch `docs/properties-panel-catalogue`. It records **current implemented behaviour only** — no proposed changes, no renames, no future/desired behaviour.

For every property/control it records: the exact UI label (i18n message id resolved to its actual English string via `messages/en.json` at the repo root), the control type, whether it is Editable / Fixed (read-only) / Action-only, available values/defaults/constraints, and the conditions under which it appears.

**On validation layers:** a control's real constraint is frequently NOT fully expressed in the Svelte component. Up to three layers can define (and disagree on) a field's limits:
1. The Svelte component (min/max/step attributes, JS clamping, `disabled` logic).
2. The Rust API validators — `#[validate(...)]` attributes in `backend/src/server/**/impl/base.rs`.
3. The database CHECK constraints in `backend/migrations/*.sql` (read in filename/chronological order — later migrations can relax or tighten earlier ones).

Where these three layers disagree, this document states all three explicitly rather than picking one. Genuine disagreements found are also summarized in [Cross-Layer Validation Disagreements](#cross-layer-validation-disagreements).

---

## 1. Panel architecture overview

The topology canvas has one floating panel container, `TopologyOptionsPanel.svelte`, which shows exactly one of the following at a time, based on selection state:

| Selection state | Component rendered |
|---|---|
| Nothing selected | `OptionsContent.svelte` (Filters / Groups / Display tabs — **out of scope for this catalogue**: it configures view filters and canvas display options, not entity properties) |
| One node selected | `InspectorNode.svelte` → `InspectorElementNode.svelte` or `InspectorContainerNode.svelte` |
| One edge selected | `InspectorEdge.svelte` → one of 8 per-kind variants, or `InspectorEdgeAggregated.svelte` |
| ≥2 nodes selected, OR an existing dependency is being edited (`editingDependencyId` set) | `InspectorMultiSelect.svelte` |
| Onboarding tutorial | `InspectorMultiSelect.svelte` in tutorial mode (`isTutorial`), or a hint string in the other slots |

A structurally separate panel, `ReadOnlyInspectorPanel.svelte`, is used when viewing a topology through a read-only share link. It does not reimplement any rendering — it reuses `InspectorNode`/`InspectorEdge` directly (see [§5](#5-read-only-share-panel-readonlyinspectorpanelsvelte)).

The custom (freeform) topology-view canvas is an entirely separate feature with its own panels: `CustomViewNodeInspector.svelte` for object/text/group nodes, `CanvasControlPanel.svelte` for canvas-level defaults, and an inline (non-componentized) edge panel in `CustomViewCanvas.svelte` (see [§6](#6-custom-topology-view-canvas-panels)).

### 1.1 Which sections a node/container inspector shows — per topology view

`InspectorElementNode.svelte`/`InspectorContainerNode.svelte` render a list of `Section*.svelte` partials chosen by `getInspectorConfig($activeView)` (`ui/src/lib/features/topology/components/panel/inspectors/view-config.ts`), which reads generated fixture data sourced from `TopologyView::inspector_config()` in `backend/src/server/topology/types/views.rs`. There are 4 views:

| View | element_sections (node selected) | container_sections (group/box selected) | dependency_creation | show_application_picker |
|---|---|---|---|---|
| **L3Logical** (default) | Identity, HostDetail, IfEntryData, Services, OtherInterfaces | SubnetDetail, ElementSummary | Bindings (forced) | false |
| **L2Physical** | Identity, IfEntryData | Identity, ElementSummary | *(none — disabled)* | false |
| **Workloads** | Identity, HostDetail, Virtualization, Services, OtherInterfaces | Identity, ElementSummary | Services | false |
| **Application** | Identity, Dependencies, Application | Identity, ElementSummary, DependencySummary | Services | true |

**Finding — two sections are currently unreachable via this routing.** The frontend's `InspectorSection` TypeScript type (`view-config.ts`) and its `SECTION_COMPONENTS` map both include `Tags` → `SectionTags.svelte` and `PortBindings` → `SectionPortBindings.svelte`. However:
- The backend Rust `InspectorSection` enum (`views.rs`) has **no `Tags` variant at all** — the backend can never emit a section value the frontend would render as `SectionTags`. The frontend type is a superset of the backend's.
- `PortBindings` **does** exist as a backend enum variant, but no `TopologyView::inspector_config()` match arm (L3Logical/L2Physical/Workloads/Application) ever includes it in `element_sections` or `container_sections`.
- Both `SectionTags.svelte` and `SectionPortBindings.svelte` are otherwise unreferenced anywhere else in `ui/src` (confirmed by search).

Both are still catalogued below (§2.13, §2.14) as "record what they would show if reached," per the audit's completeness requirement, but they are dead code in the current per-view inspector flow. (Tag editing does happen elsewhere in the app, via a separate `ui/src/lib/features/tags/` feature — `TagPicker.svelte`, `TagEditModal.svelte`, etc. — which is outside this catalogue's scope; and via the inline `TagPickerInline` embedded in several sections/panels, catalogued in place below.)

### 1.2 Entity types with a node/container inspector

Node/container inspectors resolve against these backend entity types (`EntityDiscriminants`, `backend/src/server/shared/entities.rs`), depending on view: `Host`, `Service`, `IPAddress`, `Interface`, `Subnet`. (Additional entity variants exist in the backend — `Port`, `Binding`, `Vlan`, `Dependency`, `CustomTopologyView`, `CustomViewNode`, `CustomViewEdge`, `LibraryObject`, `Tag`, etc. — but are not directly selectable as a topology node/container; several appear only as inline data within a Section, catalogued in place.)

---

## 2. Node / Container Inspector — Section partials

Scope: `ui/src/lib/features/topology/components/panel/inspectors/sections/*.svelte`, plus `shared/BindingPicker.svelte` and `shared/DependencyTargetCard.svelte`. All UI label strings are the resolved English value from `messages/en.json` for the given message id.

### 2.1 Shared building blocks used by several sections

These aren't separate sections but every section table below refers to them.

**`EntityDisplayWrapper` + `ListSelectItem` + a per-entity `*Display.svelte` config** (`HostDisplay`, `ServiceDisplay`, `IPAddressDisplay`, `InterfaceDisplay`, `SubnetDisplay`, `DependencyDisplay`, under `ui/src/lib/shared/components/forms/selection/display/`). Each config supplies `getLabel`/`getDescription`/`getIcon`/`getTags`/optionally `getTagPickerProps`/editable-description wiring; `ListSelectItem` renders the icon, label, inline tag chips, an optional entity tag-picker, and an optional inline-editable description.

- **Entity Tags picker** (`TagPickerInline.svelte`, `ui/src/lib/features/tags/components/`): a "+" button opening a search/create combobox. Rendered ONLY when the entity's Display config defines `getTagPickerProps` AND the section passes `showEntityTagPicker: true`. **Confirmed present for Host, Service, Subnet; confirmed absent for IPAddress, Interface (SNMP), Dependency** (their Display configs have no `getTagPickerProps` — those entity types can never have tags added/removed from any inspector panel today; their `getTags` don't even surface the entity's own `tags` array, only derived/synthetic tags like subnet CIDR or status).
  - Control: text input filters existing org tags by substring; matching tags show in a dropdown to click-add; typing a name with no exact match shows a `Create "{name}"` option (`tags_createTagQuoted`) if permitted.
  - Editable / Fixed: Editable, gated by `tagPickerDisabled` (from `!editState.isEditable`) AND by permission — tag *creation* additionally requires `permissions.manage_org_entities`, and is force-disabled for a non-Owner in a demo-plan organization.
  - New tag: name = typed text (required, non-empty after trim on the client), colour = **random** pick from `AVAILABLE_COLORS` (not user-chosen at creation), `is_application` set true only when the caller passed `createAsApplication` (used by `SectionApplication`).
  - Existing selected tags render as removable pill chips (`removable={!disabled}`); clicking × calls a bulk-remove-tag mutation.
  - Placeholder "Add tag..." (`tags_addTag`); "Creating..." (`common_creating`) while the create mutation is in flight.
  - **Cross-layer check on tag name**: Svelte `TagPickerInline` has **no client-side max-length or character-set validation** on the name field. Backend `TagBase.name` (`backend/src/server/tags/impl/base.rs`): `#[validate(length(min = 1, max = 100, message = "Tag name must be between 1 and 100 characters"))]`. SQL: no `CREATE TABLE tags` CHECK constraint found for `name` in `backend/migrations/*.sql`. **Disagreement**: Svelte imposes no length ceiling at all (a 500-character tag name would be typed and submitted with no client warning), while the Rust API caps it at 100 and would reject it; SQL enforces nothing beyond NOT NULL/column type.

- **Inline editable description** (`InlineDescription.svelte`, in `.../panel/inspectors/`): a `<textarea>`, default `maxLength=500`, shown only when the caller sets `showEditableEntityDescription: true` (Host, Subnet only — Service has no editable-description wiring at all, a real cross-entity asymmetry). Enter (without Shift) or blur saves; empty/whitespace-only input saves as `null`. Placeholder button when empty: "Edit description..." (`inspector_editDescription`). Editable/Fixed gated by `entityDescriptionDisabled` (`!editState.isEditable`).

### 2.2 `SectionIdentity.svelte`

Props: `node`, `topology`, `editState`, `elementContext?`, `containerContext?`. Renders a header line ("This {name}" — `inspector_thisEntity`, e.g. "This Host") with a crosshair "Focus node" icon-button (`topology_focusNode`, pans/zooms to the node — action-only). Below that, exactly ONE of the following renders depending on what resolves:

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| Section header label | Read-only text | Fixed | "This {EntityTypeName}" (e.g. "This Host", "This Service") for elements; same pattern for containers | Always shown |
| Focus-node button | Icon button | Action | Pans/zooms canvas to fit this node (300ms animated `fitView`) | Always shown; tooltip "Focus node" |
| Entity display (Interface) | `InterfaceDisplay` card | Fixed | Label, description ("No MAC Address" if none — hardcoded, not i18n), oper-status tag | Shown when `elementContext.elementType === 'Interface'` |
| Entity display (IPAddress) | `IPAddressDisplay` card | Fixed (no tag picker) | Label ("name: ip" or bare ip), description = MAC or "No MAC" (hardcoded) | Shown when `elementContext.ipAddress` resolves and no Interface match |
| Entity display (Service) | `ServiceDisplay` card | Tags editable via picker; label/description Fixed | Name, computed description (definition name / port list / binding count), Tags picker | Shown when element is a Service; in **Application** view, `ipAddressId`/`ports` context is forced null/[] so bindings never resolve to ports there |
| Entity display (Host, container) | `HostDisplay` card | Tags editable via picker; label/description Fixed | Label = host name, description = hostname or "No Hostname" (hardcoded) | Shown when `containerContext.containerType === 'Host'` and the container node has an `entity_id` |
| Container title | Read-only text | Fixed | Container's resolved title string | Shown for containers with no host-entity match (Subnet containers use `SectionSubnetDetail` instead) |

Tag-picker `disabled` on the entities above is driven by `!editState.isEditable`; tag options list is `topology.entity_tags`.

### 2.3 `SectionHostDetail.svelte`

Props: `node`, `topology`, `editState`, `elementContext?`. Only renders when `elementContext.host` resolves. Header: "Host Details" (`inspector_hostDetail`).

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| Host name / hostname | `HostDisplay` label/description | Fixed | Name = `host.name`; description = `host.hostname` or "No Hostname" | |
| Tags | Tag picker (§2.1) | Editable | Add/remove org tags on this Host | `tagPickerDisabled = !editState.isEditable`; `showEntityTagPicker` forced `true` |
| Description | `InlineDescription` textarea | Editable | Free text, max 500 chars (Svelte); saved via `useUpdateHostDescriptionMutation` | Backend `HostBase.description` also `#[validate(length(min=0,max=500))]` — **agrees**. No SQL CHECK on `hosts.description` (silent third layer, not conflicting) |
| Category | `InfoRow` read-only text | Fixed | Category name resolved from `host.category_id` | Only shown if resolvable |
| OS Group | `InfoRow` read-only text | Fixed | "{OS Group name} ({os_detail})" or whichever half is present | Only shown if resolvable |
| Manufacturer / Model | `InfoRow` read-only text | Fixed | "{manufacturer} {model}" (either half omitted if blank); row label literally "Manufacturer / Model" | Only shown if manufacturer or model present |

### 2.4 `SectionInterfaceData.svelte` → `InterfaceDetailsCard.svelte`

Props: `node`, `topology`, `elementContext?`. Resolves an SNMP `Interface` (ifEntry) row via the shared `InterfaceDetailsCard` (also reused in host detail pages). Entirely read-only: two default-expanded `CollapsibleCard`s.

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| "Status" card (collapsible) | Collapsible section | Fixed (expand/collapse is local UI state) | Header "Status" | Always shown here |
| Administrative Status | `InfoRow` | Fixed | Text from admin-status enum map, or "Unknown" | inside Status card |
| Operational Status | `InfoRow` + colored `Tag` | Fixed | Label from oper-status enum map; colour Green=Up, Red=Down, Yellow=Dormant, Gray=other/unknown | inside Status card |
| "Details" card (collapsible) | Collapsible section | Fixed | Header "Details" | default-expanded |
| ifName | `InfoRow`, label literally `ifName` (not i18n'd, not title-cased) | Fixed | `iface.if_name` or "-" | |
| ifType | `InfoRow`, label literally `ifType` | Fixed | `iface.if_type` or "-" | |
| MAC Address | `InfoRow` (monospace) | Fixed | `iface.mac_address` or "-" | |
| Speed | `InfoRow` | Fixed | Formatted bps→Gbps/Mbps/Kbps/bps (1 decimal), or "Unknown" if falsy | |
| Alias / Description | `InfoRow` | Fixed | `iface.if_alias` or "-" | |
| Index: {n} | `InfoRow`, label interpolates the index number | Fixed | `iface.if_index` shown as both label suffix and value (redundant display) | |
| IP Address | `InfoRow` | Fixed | Linked IPAddress chip, plus "on" + linked Subnet chip if resolvable; else "-" | Subnet chip shows "{name} ({cidr})" when name differs from CIDR, else bare CIDR |
| Native VLAN | `InfoRow` | Fixed | "VLAN {number} ({name})" tag | Only if a native VLAN resolves |
| Tagged VLANs | `InfoRow` | Fixed | One "VLAN {number}" tag per tagged VLAN | Only if list non-empty |
| Neighbor Device | `InfoRow` | Fixed | Neighbor Interface chip ("name/descr"/"Index {n}") + "on" + neighbor Host chip; else "-" | |

### 2.5 `SectionServices.svelte`

Props: `node`, `topology`, `editState`, `elementContext?`. One card per service bound to the current element's IP (or, for Host elements, all the host's services).

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| Section header | Read-only text | Fixed | "Services" (`common_services`) when `elementContext.elementType === 'Host'`; else "Services Bound to IP Address" (`inspector_servicesOnIPAddress`) | Section hidden entirely (no header, no cards) when zero matching services |
| Per-service card | `ServiceDisplay` (§2.1) | Tags editable; label/description Fixed | Name, computed description, Tags picker | Filtered to bindings on `elementContext.ipAddressId` (or unbound bindings) |

### 2.6 `SectionOtherIPAddresses.svelte`

Props: `node`, `topology`, `elementContext?`. Lists sibling IP addresses on the same host.

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| Section header | Read-only text | Fixed | "Other IP Address"/"Other IP Addresses" (`inspector_otherIPAddress`/`inspector_otherIPAddresses`, singular/plural) when the selected element IS an IP address; "IP Addresses" (`common_ipAddresses`) otherwise | Section hidden entirely when list is empty |
| Per-IP card | `IPAddressDisplay` (§2.1) | Fixed, no tag picker | Label, MAC/"No MAC" description | Excludes the currently-selected IP address itself when applicable |

### 2.7 `SectionSubnetDetail.svelte`

Props: `node`, `topology`, `editState`. Container-selection only (L3Logical's `container_sections`). Header: "This Subnet" (`inspector_thisSubnet`) + Focus-node icon-button (same pattern as §2.2).

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| Focus-node button | Icon button | Action | Same "Focus node" behaviour as §2.2 | |
| Name / CIDR | `SubnetDisplay` label/description | Fixed | Label = `subnet.name`; description = CIDR (blank if name equals CIDR, or if it's a container/bridge subnet) | |
| Subnet type tag | `SubnetDisplay` tag | Fixed | Shown only if the subnet type's metadata flags `show_label` | e.g. hidden for the default/plain subnet type |
| Tags | Tag picker (§2.1) | Editable | | `tagPickerDisabled = !editState.isEditable` |
| Description | `InlineDescription` textarea | Editable | Free text, max 500 chars (Svelte); saved via `useUpdateSubnetMutation` | Backend `SubnetBase.description` also `#[validate(length(min=0,max=500))]` — **agrees**. No SQL CHECK on `subnets.description` found |

### 2.8 `SectionElementSummary.svelte`

Props: `node`, `topology`. Pure read-only tally. Container-selection section in every view. Header: "Element Summary" (`inspector_elementSummary`).

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| Collective total row | Read-only text, bold, top row | Fixed | "{Collective noun, titlecased}s" : count | Only when the active view declares a `collective_noun` (currently only **Workloads**, noun "workload") |
| Per-entity-type count rows | Read-only text | Fixed | "{entity plural name}" : count, one row per element/inline entity type declared for the active view | Indented under the collective row when a collective noun exists |

### 2.9 `SectionDependencySummary.svelte`

Props: `node`, `topology`. Container-selection section (Application view's containers). Header: "Dependency Summary" (`inspector_dependencySummary`).

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| Empty state | Read-only text | Fixed | "No dependencies" (`inspector_noDependencies`) | Shown when no dependency crosses this container's boundary |
| Sub-header | Read-only text, uppercase | Fixed | "Cross-{ContainerTypeName} Dependencies" (`inspector_crossContainerDeps`) | Only shown when ≥1 crossing dependency exists |
| Per-dependency card | `DependencyDisplay` (§2.1) | Fixed, read-only, no tag picker | Name, "{n} members in dependency" description, dependency-type tag | Only dependencies with members both inside AND outside the container boundary are listed (inline services counted as "inside") |

### 2.10 `SectionApplication.svelte`

Props: `node`, `topology`, `editState`, `elementContext?`. Only renders when the element resolves to a Service. Header: "Application" (`common_application`).

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| Current application tag | Read-only shiny `Tag` chip | Fixed | Resolved application tag's name/colour | Only when the service (directly or via host inheritance) has an application tag |
| "Inherited from host" note | Read-only text | Fixed | "Inherited from host" (`tags_inheritedFromHost`) | Shown when the app tag comes from the host, not the service directly |
| Inherited override hint | Read-only text | Fixed | "Tagging directly will override the inherited application." (`tags_inheritedOverrideHint`) | Shown alongside the inherited-from-host case |
| Override note | Read-only text + second `Tag` chip | Fixed | "Overrides" (`common_overrides`) + host's app tag chip + "from host" (`tags_fromHost`) | Shown when the service has its OWN app tag differing from the host's |
| "Ungrouped" pseudo-tag | Removable `Tag` pill (not a real tag) | Action (click × opens the picker) | Label "Ungrouped" (`common_ungrouped`), grey, shiny | Shown only when the service has no resolved app tag |
| Application tag picker | `TagPickerInline`, restricted mode | Editable | Only application-flagged tags (`is_application`) offered; `allowCreate={false}` — **cannot create a new application tag from this panel**, only assign an existing one | `disabled = !editState.isEditable`; add-button hidden while the "Ungrouped" pseudo-tag is showing |

### 2.11 `SectionVirtualization.svelte`

Props: `node`, `topology`, `editState`, `elementContext?`. Shows the hypervisor Host virtualizing the current element's host, if any. Header: "Hypervisor" (`common_hypervisor`).

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| Hypervisor host card | `HostDisplay` (§2.1) | Tags editable via picker; label/description Fixed; **no editable description here** (`showEditableEntityDescription` not passed, unlike §2.3) | Name/hostname, Tags picker | Section hidden entirely if no virtualizing service/host resolves |

### 2.12 `SectionDependencies.svelte`

Props: `node`, `topology`, `elementContext?`. Only renders for Service elements. Header: "Dependencies" (`common_dependenciesLabel`).

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| Empty state | Read-only text | Fixed | "No dependencies" (`inspector_noDependencies`) | |
| "Outbound" group | Read-only text, uppercase | Fixed | "Outbound" (`common_outbound`) | Only if ≥1 outbound dependency |
| "Inbound" group | Read-only text, uppercase | Fixed | "Inbound" (`common_inbound`) | Only if ≥1 inbound dependency |
| Per-dependency card | `DependencyDisplay` (§2.1) | Fixed, read-only | Name, member-count description, dependency-type tag | No add/remove/edit control here — dependencies are created/removed via the canvas/multi-select flow (§4), not this list |

### 2.13 `SectionTags.svelte` — ⚠ not reachable via current per-view inspector routing

Props: `node`, `topology`, `editState`. Would render a standalone "Tags" (`common_tags`) header with a `TagPickerInline` targeting whatever `resolveTagTarget(node.id, node.data)` resolves to (Host or Service). **Confirmed dead**: `SECTION_COMPONENTS` in `view-config.ts` maps `Tags` → `SectionTags.svelte`, but no `TopologyView::inspector_config()` arm in `backend/src/server/topology/types/views.rs` ever includes `InspectorSection::Tags` — and the Rust `InspectorSection` enum has **no `Tags` variant at all** (§1.1). The only reference to `SectionTags` anywhere in `ui/src` is the `view-config.ts` map entry. Not a missing feature — the same tag-editing behaviour ships today inline within §2.2/§2.3/etc. via the embedded Display-config tag pickers; this is a redundant, unreachable code path.

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| Section header | Read-only text | Fixed | "Tags" (`common_tags`) | **Never rendered by any current view** |
| Tag picker | `TagPickerInline` | Editable (if reached) | Same behaviour as §2.1 | `disabled = !editState.isEditable`; target resolved via `resolveTagTarget` (Host or Service only) |

### 2.14 `SectionPortBindings.svelte` — ⚠ not reachable via current per-view inspector routing

Props: `node`, `topology`, `elementContext?`. Would render a "Port Bindings" (`common_portBindings`) card listing a Service's Port-type bindings as plain `{number}/{protocol}` monospace text. **Confirmed dead** for the same reason as §2.13, except `InspectorSection::PortBindings` DOES exist as a backend enum variant — it is simply never placed into any view's `element_sections`/`container_sections`. One `match`-arm edit away from being reachable (unlike Tags, which would also need a new backend enum variant).

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| Section header | Read-only text | Fixed | "Port Bindings" (`common_portBindings`) | **Never rendered by any current view** |
| Per-binding row | Read-only monospace text | Fixed | "{port.number}/{protocol, lowercased}", or the raw `binding.port_id` UUID if the port can't be resolved | Only Port-type bindings shown; IPAddress-type bindings filtered out |

### 2.15 `shared/BindingPicker.svelte`

Used only from `shared/DependencyTargetCard.svelte` (itself only used by `InspectorMultiSelect.svelte`, §4). Renders inside a form field as part of picking which binding (IP address or port) a service uses as one endpoint of a dependency being created.

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| IP candidate (first/caller service) | Read-only `EntityTag` chip | Fixed | Shown when exactly one candidate IP exists | `isFirstService` (flatIndex===0) branch |
| IP candidate picker (first/caller service) | Dropdown (`EntityTagSelect`) | Editable | One option per candidate IP address (deduped against IPs already claimed by another card) | Shown when >1 IP candidate; auto-resolves to the sole candidate when exactly one exists |
| "No bindings" placeholder | Read-only italic text | Fixed | "(no bindings)" (`topology_multiSelectNoBindings`) | Shown when zero IP/binding candidates exist for this service |
| Binding candidate (non-first service) | Read-only `EntityTag` chip | Fixed | Shown when exactly one port-binding candidate exists | Non-caller services must bind to a specific port, not an IP-only binding |
| Binding candidate picker (non-first service) | Dropdown (`EntityTagSelect`), placeholder "Select a port..." (`dependencies_selectPort`) | Editable | One option per open-port binding not already claimed elsewhere in the form | |
| "No open ports" error | Read-only error text (red) | Fixed | "{serviceName} doesn't have any open ports. Move it to the first position, remove it, switch to \"Services only\", or rerun discovery on {hostName} to discover open ports." (`dependencies_noOpenPortsError`) | Shown when the service's only bindings are IP-only and it isn't the first/caller service |

### 2.16 `shared/DependencyTargetCard.svelte`

Also multi-select-only (§4). One card per target service/host/IP-address being assembled into a new dependency.

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| "Host" row | Read-only label + `EntityTag` chip | Fixed | Literal label "Host" (`common_host`) + linked Host chip | |
| "running" row label | Read-only text | Fixed | Literal, hardcoded English word **"running"** — not resolved via a message id, unlike almost every other label in this file | Precedes the resolved/pickable service |
| Service picker | Dropdown (`EntityTagSelect`) | Editable | Offered when the target (a Host or IPAddress target with >1 candidate service) needs disambiguation | Auto-seeds the first candidate as default |
| Service chip (resolved, single candidate) | Read-only `EntityTag` chip | Fixed | | |
| "No services" error | Read-only error text (red) | Fixed | "{name} has no discovered services. Run a discovery scan first." (`dependencies_noServicesError`) | Shown when a Host/IPAddress target resolves to zero candidate services |
| "at" row label | Read-only text | Fixed | Resolved message `common_at` = "at" | Precedes the `BindingPicker` in "Bindings" dependency-member mode |
| Binding picker | `BindingPicker` (§2.15) | Editable | | Only shown when `memberMode === 'Bindings'` |
| Remove (×) button | Icon button | Action | Removes this target card from the dependency being built (and, in create mode, from the canvas selection) | Tooltip/aria-label "Remove" (`common_remove`); only rendered when `onRemove` is supplied |

### 2.17 Section partials — notes

- **Entity tag picker coverage is inconsistent by entity type**: Host, Service, Subnet get a tag picker; IPAddress, Interface (SNMP), Dependency do not — those three entity types can never have tags added or removed from any inspector panel.
- **Untranslated / hardcoded English strings found** (will not localize): `HostDisplay.getDescription` fallback "No Hostname"; `IPAddressDisplay.getDescription` fallback "No MAC"; `InterfaceDisplay.getDescription` fallback "No MAC Address" (inconsistent with IPAddressDisplay's "No MAC" — two different fallback strings for the same "no MAC known" concept); `InterfaceDetailsCard`'s raw field labels `ifName`/`ifType`; `DependencyTargetCard`'s row label "running".
- **Service has no editable description field anywhere in the inspector** — `ServiceDisplayContext` has no `showEditableEntityDescription`/`entityDescription`/`onEntityDescriptionSave` props at all, unlike `HostDisplayContext` and `SubnetDisplayContext`.

---

## 3. Edge Inspector — `InspectorEdge.svelte` and its 8 kind variants

Source: `ui/src/lib/features/topology/components/panel/inspectors/InspectorEdge.svelte` and `edges/*.svelte`.

### 3.1 `InspectorEdge.svelte` (dispatcher)

Not a panel itself — routes to one of the variants below based on `edge.data.edge_type` (or to `InspectorEdgeAggregated` when the selection bundles multiple original edges under one drawn edge, `edgeData.isAggregated === true`).

| edge_type value | Component rendered | Notes |
|---|---|---|
| `HubAndSpoke` or `RequestPath` | `InspectorEdgeDependency` | Both dependency-edge kinds share one component. |
| `SameHost` | `InspectorEdgeIPAddress` | **Naming mismatch**: the edge type is `SameHost` but the component file is named `InspectorEdgeIPAddress.svelte` — record both names. |
| `Hypervisor` | `InspectorEdgeHypervisor` | |
| `ContainerRuntime` | `InspectorEdgeContainerRuntime` | |
| `SameContainer` | `InspectorEdgeSameContainer` | |
| `PhysicalLink` | `InspectorEdgePhysicalLink` | |
| `NeighborLink` | `InspectorEdgeNeighborLink` | |
| (aggregated bundle) | `InspectorEdgeAggregated` | Takes precedence over the per-kind routing above whenever the selected drawn edge represents >1 original edge. |
| anything else / no `edgeData` | fallback text | Two fallback strings, both read-only, no controls: if `edgeData` is falsy → `inspector_edgeDataUnavailable` = **"Edge data not available"**; if `edgeData` is truthy but matches no known `edge_type` → `inspector_edgeDetailsUnavailable` = **"Unable to display edge details"**. |

Which edge types can even appear, and whether they start visible, is entirely a per-`TopologyView` backend setting (`TopologyView::edge_view_config()` in `backend/src/server/topology/types/views.rs`), reproduced here since it directly gates when a reader will ever see each panel below:

| Edge type | L3Logical | L2Physical | Workloads | Application |
|---|---|---|---|---|
| SameHost | Active, **visible** by default | Active, hidden by default | Disabled (cannot appear) | Disabled |
| ContainerRuntime | Active, **visible** | Disabled | Disabled | Active, hidden by default |
| RequestPath | Active, **visible** | Disabled | Active, hidden by default | Active, **visible** |
| HubAndSpoke | Active, **visible** | Disabled | Active, hidden by default | Active, **visible** |
| Hypervisor | Active, hidden by default | Disabled | Disabled | Disabled |
| PhysicalLink | Disabled | Active, **visible** | Active, hidden by default | Disabled |
| NeighborLink | Disabled | Active, **visible** | Active, hidden by default | Disabled |
| SameContainer | Active, hidden by default | Disabled | Disabled | Disabled |

"Disabled" = this edge type cannot be drawn at all in that view (its inspector panel is therefore unreachable there). "Hidden by default" = the edge exists and its inspector is reachable once the user toggles it on, but it is not shown out of the box.

All edge-kind panels below are driven by `getTopologyEditState(topology, false, isReadonly)`, where `isReadonly` comes from `topo.isReadonly` (share context) or the `topologyReadOnly` store. `isEditable` is simply `topology loaded && !isReadonly` (`ui/src/lib/features/topology/state.ts:20-27`) — there is no separate per-field permission check inside these edge panels.

**Shared embedded control — Entity tag picker.** Six of the eight variants (all except `InspectorEdgeNeighborLink` and `InspectorEdgePhysicalLink`) render their Host/Service/Subnet cards with `showEntityTagPicker: true`, `tagPickerDisabled: !editState.isEditable`. This renders an inline add/remove tag control (`TagPickerInline.svelte`, reached via `ListSelectItem.svelte`) on that entity's card — Editable (add/create/remove tags) when the topology is not read-only, Fixed/disabled otherwise. **Inconsistency**: `InspectorEdgeNeighborLink` and `InspectorEdgePhysicalLink` show Host/Interface cards WITHOUT `showEntityTagPicker` — tags cannot be added/removed from those cards in this panel, unlike every other edge kind's host/service cards. Recorded as current behavior only, not a bug assessment.

### 3.2 `InspectorEdgeAggregated.svelte`

Shown instead of any per-kind panel when the drawn edge bundles multiple original edges (e.g. many parallel edges of possibly different types collapsed into one visual line).

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| Connections count header | Read-only text | Fixed | `topology_connectionsCount` = **"{count} connections"**, count = number of underlying edges | Always shown |
| Per-type group header | Read-only text | Fixed | Group label + count, e.g. "{type} (N)"; dependency edge types (HubAndSpoke/RequestPath) are labelled `common_dependenciesLabel` = **"Dependencies"** instead of their raw type name; other types use `edgeTypes.getName(edgeType)` (metadata-store display name) | One header per distinct `edge_type` present in the bundle |
| Dependency entries | Read-only entity card (`DependencyDisplay`) | Fixed | One card per unique `dependency_id` among HubAndSpoke/RequestPath edges in the bundle, each followed by nested cards for its member services/bindings | Only when the group is a dependency-edge type |
| ContainerRuntime — single containerizer | Read-only entity cards | Fixed | Header `inspector_dockerService` = **"Docker Service"** for the containerizing service card, then `common_containerizedService`/`common_containerizedServices` = **"Containerized Service"/"Containerized Services"** (count-based) header + one card per containerized service | Shown when all ContainerRuntime edges in the bundle share one `service_id` |
| ContainerRuntime — multiple containerizers | Read-only entity cards | Fixed | One card per host, each with a `Tag` labelled `common_docker` = **"Docker"** (color: Indigo) and a count string `topology_containerCount` = **"{count} containers"** | Shown when more than one distinct containerizing `service_id` is present |
| Other edge kinds (SameHost/PhysicalLink/NeighborLink/Hypervisor) | Read-only entity card via a per-type display component, or plain type-name text if no display component is registered | Fixed | `SameHost`→`SameHostEdgeDisplay`, `PhysicalLink`/`NeighborLink`→`PhysicalLinkEdgeDisplay`, `Hypervisor`→`HypervisorEdgeDisplay`; anything else falls back to a bare `{typeName}` line | |

No editable controls or action buttons anywhere in this component — pure grouped read-only summary.

### 3.3 `InspectorEdgeContainerRuntime.svelte`

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| "Container Host" (`topology_containerHost`) | Section label + read-only `HostDisplay` card | Fixed, embeds tag picker | The host running the containerizing service | Only shown if resolvable |
| "Container Service" (`topology_containerService`) | Section label + read-only `ServiceDisplay` card | Fixed, embeds tag picker | The containerizing (e.g. Docker daemon) service | Only shown if resolvable |
| "Containerized Service"/"Containerized Services" (`common_containerizedService`/`common_containerizedServices`) | Section label (singular/plural by count) + list of `ServiceDisplay` cards | Fixed, embeds tag picker | One card per container reached by this edge (via `edge.data.containerized_service_ids`) | Always rendered (label + zero-or-more cards) |
| "Container Bridge Subnet"/"Container Bridge Subnets" (`topology_containerBridgeSubnet`/`topology_containerBridgeSubnets`) | Section label + `SubnetDisplay` cards | Fixed | One card per bridge subnet in `edge.data.subnet_ids` | Only shown if ≥1 bridge subnet resolved |
| Tag picker on Host/Service cards | Inline add/remove control (`TagPickerInline`) | **Editable** when `editState.isEditable` (topology loaded & not read-only), else disabled | Add/remove organization tags on that Host or Service entity | See §3.1 shared note |

No buttons/actions of its own (no edit/delete) — this edge kind is fully derived from discovery data.

### 3.4 `InspectorEdgeDependency.svelte`

Renders for `HubAndSpoke` and `RequestPath` edge types — the only edge kind in this family with true CRUD actions on the underlying record.

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| "Dependency" (`common_dependency`) card | Read-only entity card (`DependencyDisplay`): name, member count, icon by `dependency_type`, tag (non-compact) showing dependency type name | Fixed display, but embeds an editable inline description (next row) | Card shows `dependency.name` as label and `"{count} member(s) in dependency"` as description-fallback text | |
| Dependency description | Inline editable text (`showEditableEntityDescription` context flag on the `DependencyDisplay` card, rendered via `InlineDescription.svelte`) | **Editable** | Free text, saved via `useUpdateDependencyDescriptionMutation` on save (`onEntityDescriptionSave`) | Disabled (`entityDescriptionDisabled`) when `!editState.isEditable`; initial value = `group.description` (nullable). **Backend**: `DependencyBase.description` — `#[validate(length(min = 0, max = 500))]` (`backend/src/server/dependencies/impl/base.rs`). **SQL**: `dependencies.description` (renamed from `groups.description`) is plain nullable `TEXT` with no CHECK constraint (`backend/migrations/20251006215201_create_groups.sql`). No Svelte-side max-length attribute was found on the inline editor — three layers effectively agree only on "no hard ceiling enforced end-to-end below 500," since the DB imposes none and the Svelte editor imposes none either; only the API layer caps it at 500. |
| "Edit" button (pencil icon, `common_edit` = **"Edit"**) | Button / Action | Action-only | Pushes the dependency's member entities into the canvas selection (view-aware mapping — Service IDs directly in Workloads/Application, or the owning IPAddress node in L3) and sets `editingDependencyId`, switching the panel to `InspectorMultiSelect` in edit mode (§4) | Only rendered when `!isReadonly && editState.isEditable` |
| "Delete" button (trash icon, `common_delete` = **"Delete"**, shows `common_deleting` = **"Deleting..."** while pending) | Button / Action | Action-only | On click, shows a native `confirm()` dialog with `common_confirmDeleteName` = **"Are you sure you want to delete \"{name}\"?"**; on confirm, calls `useDeleteDependencyMutation` and clears the topology selection | Only rendered when `!isReadonly` (note: NOT additionally gated on `editState.isEditable`, unlike the Edit button and the tag pickers elsewhere — a real asymmetry in this file, recorded as-is) |
| "Services" (`common_services`) member list | Read-only cards | Fixed (cards themselves are read-only; membership is changed via the Edit flow above, not inline here) | If `members.type === 'Bindings'`: one `BindingWithServiceDisplay` card per binding, ring-highlighted (source/target ring colors differ for RequestPath vs other dependency types — RequestPath both ends ring gray with a down-arrow between them showing flow direction; non-RequestPath source rings the dependency's own color, target rings gray). If `members.type === 'Services'`: one `ServiceDisplay` card per service, no highlight rings | |
| Dependency `color` / `edge_style` | *(no control in this file)* | N/A here | The component reads `group.color`/`group.edge_style` into `localGroup` and auto-saves via `useUpdateDependencyMutation` whenever they differ from `group`, but no input in this file ever sets those fields — the auto-save `$effect` is dead code as far as this component's own UI goes. **Confirmed** (cross-checked against §4): the actual color/style controls live in `InspectorMultiSelect.svelte`'s dependency edit form (`EdgeStyleForm` — §4, "Edge color picker"/"Edge style picker" rows), reached via this panel's Edit button. | Auto-save only fires when not read-only (`!isReadonly`) |

### 3.5 `InspectorEdgeHypervisor.svelte`

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| "Hypervisor Service" (`hosts_virtualization_hypervisorService`) | Section label + `ServiceDisplay` card | Fixed, embeds tag picker | The service representing the hypervisor | Shown if resolvable |
| "Hypervisor Host" (`inspector_hypervisorHost`) | Section label + `HostDisplay` card | Fixed, embeds tag picker | The physical host running the hypervisor (`edge.target`) | Shown if resolvable |
| "Virtual Machines" (`hosts_virtualization_virtualMachines`) | Section label + list of `HostDisplay` cards | Fixed, embeds tag picker per card | All hosts where `host.virtualization_service_id === hypervisorServiceId` | If none, shows `hosts_virtualization_noVmsYet` = **"No VMs managed by this service yet. Add hosts that are VMs running on this hypervisor."** |

No action buttons.

### 3.6 `InspectorEdgeIPAddress.svelte` *(handles the `SameHost` edge type — see §3.1 naming-mismatch note)*

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| "Host" (`common_host`) | Section label + `HostDisplay` card | Fixed, embeds tag picker | The shared host both IP addresses sit on | Shown if resolvable |
| "IP Addresses" (`common_ipAddresses`) | Section label + up to 2 `IPAddressDisplay` cards | Fixed | Source and target IP-address entities (`edge.source`/`edge.target`), each with subnet context | Cards render only for whichever of source/target resolves |

No action buttons; `view` prop accepted but unused in the template (eslint-disabled unused-var, same pattern as `InspectorEdgeDependency`).

### 3.7 `InspectorEdgeNeighborLink.svelte`

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| Protocol tag | Read-only colored `Tag` | Fixed | Value is `LLDP` or `CDP` (verbatim, not translated); color CDP="Blue", LLDP="Green" | Only shown if `protocol` prop set |
| Informational note | Read-only text | Fixed | `topology_neighborLinkPortsUnknown` = **"These devices report each other as neighbors, but the ports connecting them could not be identified."** | Always shown |
| "Source" (`common_source`) | Section label + `HostDisplay` card | Fixed | `sourceHostId` resolved to a Host, with its services listed as card context | No tag picker (§3.1 inconsistency) |
| "Target" (`common_target`) | Section label + `HostDisplay` card | Fixed | `targetHostId` resolved to a Host | No tag picker |

No action buttons.

### 3.8 `InspectorEdgePhysicalLink.svelte`

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| Protocol tag | Read-only colored `Tag` | Fixed | `LLDP`/`CDP` verbatim; same color mapping as NeighborLink | Only if `protocol` set |
| "Neighbor report" (`topology_neighborEvidence`) row | Read-only text + relative-time value + optional evidence `Tag` | Fixed | Timestamp is the more-recent of the two interfaces' `neighbor_seen_at`, formatted via `formatRelativeTime`; the small pill tag next to it is computed by `neighborEvidenceTag(evidenceEndpoint, evidenceNetwork)` (freshness/staleness indicator) | Only shown if at least one endpoint interface has `neighbor_seen_at` set |
| "Source" (`common_source`) | Section label + `HostDisplay` card (no tag picker) + `InterfaceDisplay` card | Fixed | Source interface's host, then the interface itself | Rendered only for whichever pieces resolve |
| "Target" (`common_target`) | Section label + `HostDisplay` card (no tag picker) + `InterfaceDisplay` card | Fixed | Target interface's host, then the interface itself | |

No action buttons.

### 3.9 `InspectorEdgeSameContainer.svelte`

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| "Containerized Service" (`common_containerizedService`) | Section label + `ServiceDisplay` card | Fixed, embeds tag picker | The container/service this edge represents | Shown if resolvable |
| "Container Bridge Subnets" (`topology_containerBridgeSubnets`) | Section label + `SubnetDisplay` cards | Fixed, embeds tag picker per card | Every subnet reachable by this container's bindings where the subnet type metadata flags `is_container_bridge` | Only shown if ≥1 such subnet found |

No action buttons.

### 3.10 Edge inspector — notes

- **True editability is rare in this whole family.** Of 8 variants, only `InspectorEdgeDependency` has real record mutation (description edit, Edit→multi-select, Delete). The rest are 100% read-only displays of derived/discovered graph data, except for the shared entity-card tag picker.
- Whether the missing tag picker on `InspectorEdgeNeighborLink`/`InspectorEdgePhysicalLink` host cards is intentional was not determined — recorded purely as an observed difference from the other 6 variants.

---

## 4. Multi-Select / Dependency Composer Panel — `InspectorMultiSelect.svelte`

File: `ui/src/lib/features/topology/components/panel/inspectors/InspectorMultiSelect.svelte` (1149 lines) — the largest inspector component.

Shown when 2+ nodes are selected on the standard topology canvas, OR when editing an existing dependency (`editingDependency` prop set from §3.4's Edit button — `$selectedNodes` ignored in that mode), OR during the multi-select step of the onboarding tutorial (`isTutorial`). It is **one component with conditional regions**, not several sub-panel variants — which regions render depends on `editState.isEditable`, `inspectorConfig.show_application_picker` (view-driven), `inspectorConfig.dependency_creation` (view-driven: `null`/`Services`/`Bindings`), and `isTutorial`/`isEditMode`.

Sub-components used: `TagPickerInline`, `EdgeStyleForm` (color + edge-style picker), `DependencyTargetCard` (§2.16), `BindingPicker` (§2.15), `SegmentedControl`, `EntityTagSelect`.

### 4.1 Header row (always shown when not editable-gated)

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| Selection count | Read-only text | Fixed | "{summary} selected" (`appWizard_selectedCount`), summary built by `formatEntityCounts(tallySelection(nodes))`, e.g. "3 services, 1 host" | Always shown |
| Focus selection button | Icon button (Crosshair) | Action | Zooms/pans the canvas to the selection (`fitView`) | Hidden when `isTutorial` |
| Clear selection button | Icon button (X) | Action | Clears the multi-selection | Hidden when `isTutorial` |

### 4.2 Read-only state

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| Read-only hint text | Read-only text | Fixed | "This is a read-only view." (`topology_multiSelectReadOnlyHint`) | Shown instead of all editing UI below when `editState.isEditable` is false — only shown if `editState.disabledReason` is set |

### 4.3 Tags section (per taggable entity-type group)

One block per distinct resolved taggable entity type in the selection (`Host` and/or `Service` — IPAddress/Interface elements resolve up to their parent Host via `resolveTagTarget`). Entire block (and the app picker below) is skipped when `isTutorial`.

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| Group header | Read-only text | Fixed | "Common {entity} tags" (`tags_entityTags`, e.g. "Common services tags") | One per TagGroup (Host, Service) present in selection |
| No-common-tags hint | Read-only text (italic) | Fixed | "Selected {entity} have no tags in common. Pick a tag to add it to all of them." (`tags_noCommonTagsHint`) | Shown only when the group's common non-app tags list is empty AND the group has >1 entity |
| Tag picker (`TagPickerInline`) | Multi-select tag chips with add (+) affordance | Editable | Chips = tags common to ALL entities in the group (excluding application tags); adding calls a bulk-add mutation (`entity_ids`, `entity_type`, `tag_id`); removing calls bulk-remove | One instance per TagGroup |
| "Create grouping rule from {tag}" button | Button + tag chip(s) | Action | Label `inspector_createGroupingRuleFromTag`, followed by the just-added tag chip(s); clicking appends a `ByTag` element rule to the topology's shared `element_rules` options (persisted via debounced options-store PUT) | Only shown after a tag was just added in this session AND no existing rule already covers those tag ids |

### 4.4 Application picker (view-conditional)

Shown only when `inspectorConfig.show_application_picker` is true — per `views.rs`, **Application view only**. Computed off `selectedServices` only, never hosts.

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| Section header | Read-only text | Fixed | "Application" (`common_application`) | |
| Cross-group hint | Read-only text | Fixed | "To change application, select services from the same application." (`tags_crossGroupSelectionHint`) | Shown instead of the picker when selected services belong to more than one distinct application tag (or a mix of tagged/untagged) |
| Inherited-app tag chip | Read-only "shiny" tag chip | Fixed | App tag name/color inherited from the host, plus "Inherited from host" (`tags_inheritedFromHost`) and "Tagging directly will override the inherited application." (`tags_inheritedOverrideHint`) | Shown only when every selected service's app membership is inherited from its host |
| "Ungrouped" pseudo-tag chip | Removable tag chip (gray, "shiny", pill) | Action (dismiss-only, not a real tag) | Label "Ungrouped" (`common_ungrouped`); clicking its × opens the tag picker and hides the pseudo-chip for the rest of the session | Shown when none of the selected services have any app tag and not yet dismissed |
| App tag picker (`TagPickerInline`) | Single-effective-select dropdown of application-flagged tags | Editable | Selecting bulk-adds it to all selected services; removing bulk-removes. `allowCreate={false}`. Options restricted to the currently-common app tag when one is set | Add-button hidden while selection is Ungrouped-and-not-dismissed |

### 4.5 Dependency creation / edit form

Shown when `inspectorConfig.dependency_creation` is non-null for the active view, or always when editing an existing dependency (`isEditMode`). Per `views.rs`: L3Logical → `Bindings` (forced), Application → `Services`, Workloads → `Services`, L2Physical → `null` (dependency creation unavailable in L2).

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| Section header | Read-only text | Fixed | "Create Dependency" (`dependencies_createDependency`) or "Edit Dependency" (`dependencies_editDependency`) in edit mode | |
| Dependency type toggle | Segmented control (2 icon options) | Editable | `RequestPath` / `HubAndSpoke` (icons + names from `dependencyTypes` metadata store) | Default `RequestPath` for new dependencies; seeded from existing value in edit mode |
| Preview-edge toggle button | Icon button (Eye/EyeOff) | Action / persisted UI preference | "Show preview"/"Hide preview" (`topology_showPreview`/`topology_hidePreview`), caption "Preview Edge" (`topology_multiSelectPreviewEdge`) | State persisted to `localStorage` key `scanopy_topology_group_preview` — a UI preference, not part of the dependency record |
| Dependency name | Text input | Editable | Label "Dependency name" (`common_entityName` + `common_dependency`); placeholder same. Auto-filled with a generated name (`generateDependencyName`) from type + selected node names unless the user has typed a custom value. | **Frontend**: `required` (non-empty) on blur, then `max(100)` chars. **Backend** (`DependencyBase.name`, `backend/src/server/dependencies/impl/base.rs`): `#[validate(length(min = 0, max = 100))]` — max agrees (100), but **min = 0 permits an empty string server-side**. **SQL**: `dependencies.name` (`groups.name` originally, `backend/migrations/20251006215201_create_groups.sql`) is `TEXT NOT NULL` — no length CHECK, and NOT NULL does not prevent an empty string. **Disagreement: frontend requires a non-empty name; backend and DB both allow one.** |
| Edge color picker (`EdgeStyleForm`, collapsed by default) | Swatch grid (7-per-row) + collapsed summary swatch | Editable | 18 named colors (`AVAILABLE_COLORS`): Pink, Rose, Red, Amber, Orange, Yellow, Green, Emerald, Teal, Cyan, Blue, Indigo, Purple, Fuchsia, Violet, Sky, Gray, Lime. Default: random pick for new dependencies; seeded from `editingDependency.color` in edit mode. Header "Edge Color" (`dependencies_edgeColor`), help "Choose the color for edges in this dependency" (`dependencies_edgeColorHelp`) | Collapsed view shows a swatch + "Style: {label}" / "Color: {ColorName}"; expand via pencil icon (aria-label `dependencies_editEdgeStyle` = "Edit Edge Style"). Backend: `Dependency.color: Color` — typed enum, rejected at deserialization if invalid, no separate `#[validate]` needed. SQL: no CHECK on `dependencies`/`groups` color column. |
| Edge style picker (same `EdgeStyleForm`) | Radio-style button list (3 options) | Editable | "Straight" (`common_straight`), "Smooth Step" (`dependencies_smoothStep`), "Bezier" (`common_bezier`). Default `Bezier` set by `InspectorMultiSelect` for new dependencies. | Header "Edge Style" (`dependencies_edgeStyleLabel`), help "Choose how edges are drawn between services" (`dependencies_edgeStyleHelp`). **Internal inconsistency (not cross-layer)**: `EdgeStyleForm`'s own internal fallback default is `'SmoothStep'` if the bound value is falsy, disagreeing with the parent's `'Bezier'` default — but the parent always seeds a concrete value, so the child fallback is dead in this flow. |
| Member mode toggle ("Services only" / "With ports") | Segmented control (2 options) | Editable | "Services only" (`dependencies_servicesOnly`) vs "With ports" (`dependencies_withPorts`); contextual hint text when active view ≠ L3Logical | **Hidden entirely** when `isTutorial` or when the view forces bindings (L3Logical — forced to `Bindings` via an effect) |
| Per-target service card (`DependencyTargetCard`, §2.16) | Composite card | Mixed | One per selected element resolved to a dependency target, in canvas selection order | "Hub"/"Spokes" labels (`common_hub`/`common_spokes`) for `HubAndSpoke`; "↓ makes a request to" (`common_makesRequestTo`) between every pair for `RequestPath`; "↓ serves" (`common_serves`) between hub and spokes for `HubAndSpoke` |
| Remove-target (X) button on a card | Icon button | Action | Removes that target from the dependency being composed | Only shown when there are currently >2 targets — cannot shrink below 2 members via this control |
| Cancel button | Button | Action | "Cancel" (`common_cancel`) — discards edit and clears preview | Edit mode only |
| Create/Update submit button | Button | Action | "Create Dependency" (`dependencies_createDependency`) / "Update" (`common_update`) | Disabled (`canCreate` false) unless: name non-empty, no mutation in flight, ≥2 resolved services, no target with zero candidate services, and (if Bindings mode) every service has a chosen binding |
| Tutorial "Finish" button | Button | Action | "Finish Tutorial" (`topology_tutorialDone`) — replaces Create/Cancel/Update entirely | `isTutorial` only |

### 4.6 Backend / DB cross-check summary

- `DependencyBase`: `name` — `#[validate(length(min = 0, max = 100))]`; `description` — `#[validate(length(min = 0, max = 500))]` (not exposed in this panel — no description field is editable here, only via §3.4's inline description). `color`/`edge_style` are typed Rust enums (rejected at deserialization, not via `#[validate]`).
- SQL: no CHECK constraints found on `dependencies.name`/`color`/`edge_style` across migrations — NOT NULL only.
- **Disagreement**: frontend requires a non-empty dependency name; backend validator explicitly permits empty (`min = 0`); DB has no length/emptiness constraint at all. Three layers, three different effective rules.

### 4.7 Multi-select — notes

- This component is single-panel/multi-region, not several independent sub-panels.
- Did not chase `TagPickerInline`/`EntityTagSelect`'s own internal tag-creation validation beyond what §2.1/§2.17 already record.

---

## 5. Read-only Share Panel — `ReadOnlyInspectorPanel.svelte`

File: `ui/src/lib/features/shares/components/ReadOnlyInspectorPanel.svelte`. Shown when viewing a topology through a read-only share link.

### 5.1 How it's built

It does **not** reimplement Identity/HostDetail/Services/etc. rendering — it directly imports and reuses `InspectorNode`/`InspectorEdge` from the topology feature verbatim. Every field documented in §2/§3 applies here unchanged.

Read-only behavior is not implemented by this component stripping anything; it comes from two shared, global mechanisms its ancestor sets:

1. **`topologyReadOnly`** — a module-level `writable(false)` store (`ui/src/lib/features/topology/queries.ts:411`). The parent `ReadOnlyTopologyViewer.svelte` sets it `true` on mount, `false` on destroy. `InspectorElementNode`/`InspectorContainerNode` compute `isReadonly = topo.isReadonly || $topologyReadOnly` and feed it into `getTopologyEditState(...)`, which every section receives as `editState` — the single switch that disables edit affordances throughout the section tree.
2. **`staticTags`** — a Svelte **context** (not a store), `setContext('staticTags', true)`, set by `ReadOnlyInspectorPanel.svelte` itself. Read by `EntityTag.svelte`/`ListSelectItem.svelte` to suppress interactive tag affordances (hover/click, the removable-tag ×) — independent of `topologyReadOnly`.

### 5.2 Panel-level table

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| Panel expand/collapse chevron | Button/action | Action-only | Toggles local `expanded` state; also writes the global `optionsPanelExpanded` store | Auto-expands whenever `selectedNode` or `selectedEdge` becomes non-null |
| Collapse-button aria-label | Read-only text (accessibility only) | Fixed | `topology_collapsePanel` = **"Collapse panel"** | Only present while expanded |
| Expand-button aria-label | Read-only text (accessibility only) | Fixed | `topology_expandPanel` = **"Expand panel"** | Only present while collapsed |
| Panel title | Read-only text | Fixed | Literal hardcoded string **"Inspector"** (with an `Info` icon) — **not** routed through a paraglide message, unlike the collapse/expand aria-labels in the same file | Only shown while expanded |
| Empty-selection placeholder | Read-only text | Fixed | Literal hardcoded string **"Click on a node or edge to inspect it"** — also not an i18n message | Shown when expanded and neither node nor edge is selected |
| Node/edge detail body | Composed component (delegates entirely) | Mixed (inherits from `InspectorNode`/`InspectorEdge`) | Renders `<InspectorNode node={$selectedNode}/>` or `<InspectorEdge edge={$selectedEdge}/>`, keyed on id | See §2/§3 for full field-level content, all with `topologyReadOnly=true` |

No field in `ReadOnlyInspectorPanel.svelte` itself is an editable control — the only interactive elements are the two expand/collapse chevrons, pure UI-state toggles, not data edits.

### 5.3 Differences vs. the editable topology inspector panels

The editable counterpart is a structurally different component, `TopologyOptionsPanel.svelte` (§1), which `ReadOnlyInspectorPanel.svelte` only partially mirrors:

1. **No Filter / Groups / Display tabs.** `TopologyOptionsPanel` shows three tabs — "Filters" (`common_filters`), "Groups" (`common_groupsLabel`), "Display" (`common_display`) — and renders `OptionsContent.svelte` when nothing is selected. `ReadOnlyInspectorPanel` has no tabs and no options content at all; when nothing is selected it only shows the hardcoded placeholder text above. The entire Filters/Groups/Display surface is unavailable in a read-only share.
2. **No multi-select bulk-edit panel.** `TopologyOptionsPanel` renders `InspectorMultiSelect` (§4) whenever ≥2 nodes are selected or a dependency is being edited. `ReadOnlyInspectorPanel` never reads `selectedNodes`/`editingDependencyId` at all — multi-selection and the whole dependency-composer panel are unreachable from the share view.
3. **No tutorial mode.** `TopologyOptionsPanel` has an `isTutorial` branch (different z-index, a tutorial hint message, no tabs/collapse button). `ReadOnlyInspectorPanel` has no equivalent.
4. **Header content differs.** When something is selected, `TopologyOptionsPanel`'s header shows only the collapse chevron (no title text). `ReadOnlyInspectorPanel`'s header always shows an `Info` icon plus the literal text "Inspector" alongside the chevron, whether or not something is selected.
5. **Width mechanism differs (same visual result).** `TopologyOptionsPanel` sets inline `width: 320px` (`OPTIONS_PANEL_WIDTH_PX` constant) when expanded; `ReadOnlyInspectorPanel` applies Tailwind class `w-80` (also 320px by default) — different mechanism, coincidentally identical width.
6. **Stacking (z-index) differs.** `TopologyOptionsPanel` uses `z-30` in tutorial mode, `z-10` otherwise. `ReadOnlyInspectorPanel` is always `z-10`.
7. **A second, independent "read-only" signal exists.** `TopologyOptionsPanel` itself accepts an `isReadOnly` prop (default `false`) forwarded only into `InspectorMultiSelect` — a third read-only code path, unexercised in the share view since multi-select is unreachable there. "Read-only" is implemented via at least three independent signals (`topologyReadOnly` store, `staticTags` context, this `isReadOnly` prop), not one unified flag.
8. **Minimap-aware max-height differs slightly.** `ReadOnlyInspectorPanel`'s content `max-height` subtracts `MINIMAP_FITVIEW_BOTTOM_PX + 20` when the minimap is shown, else `180`. `TopologyOptionsPanel` uses a flat `180` unless `!isTutorial && !$topologyOptions.local.show_minimap` is false, in which case `350` — a different formula, not merely a read-only toggle of the same computation. Cosmetic scroll-area sizing only, not a data/field difference.
9. **Localization gap specific to the share panel.** The static title "Inspector" and the empty-state hint "Click on a node or edge to inspect it" are hardcoded English strings, unlike almost every other user-facing string in both this file and its editable counterpart.
10. **Nothing is shown in the read-only panel that the editable one doesn't also show** — it is a strict subset (same node/edge detail rendering, minus tabs, minus multi-select, minus tutorial, minus a couple of chrome strings/labels).

No evidence of field-level stripping for privacy/security (no internal IDs or admin-only data specifically hidden by this component) — it doesn't touch field rendering at all; any such stripping would live inside the Section components themselves (§2), gated on `editState`/`isReadonly`, and none was found there beyond disabling edit controls.

---

## 6. Custom Topology View Canvas Panels

Scope: `ui/src/lib/features/topology/components/visualization/custom/` — the freeform "custom topology view" canvas (distinct from the discovery-driven L2/L3/Workloads/Application inspectors). Three real properties panels exist here:

1. **`CustomViewNodeInspector.svelte`** — shown when a canvas object/text/group node is selected.
2. **`CanvasControlPanel.svelte`** — canvas-level settings + style defaults, shown via a "Settings" toggle, independent of node/edge selection.
3. **An inline "Join settings" panel in `CustomViewCanvas.svelte`** (not a separate component file) — shown when a connector/edge is selected. `CustomViewEdge.svelte` itself is a pure renderer with no inputs at all; the edit UI for edges lives inline in the canvas component.

Backend: `backend/src/server/custom_view_nodes/impl/base.rs` (`CustomViewNodeBase`), `backend/src/server/custom_topology_views/impl/base.rs` (`CustomTopologyViewBase`), `backend/src/server/custom_view_edges/impl/base.rs` (`CustomViewEdgeBase`).

### 6.1 `CustomViewNodeInspector.svelte`

Floating panel, top-right of canvas. Header: `"{node.kind} node{ — libraryObjectName if kind=Library}"` (e.g. "Entity node", "Library node — Router"), plus a Delete button (trash icon, title = `common_delete()` → **"Delete"**).

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| Panel title (`{kind} node — {libraryObjectName}`) | Read-only text | Fixed | Literal node kind + resolved library object name | `libraryObjectName` only resolved/shown when `kind === 'Library'` |
| Delete (trash icon button) | Button/Action | Action | Deletes the node | title = i18n `common_delete` → "Delete" |
| Label | Text input | Editable | Free text, HTML `maxlength=200`. Committed on blur (draft-then-commit pattern); Enter blurs/commits, Escape reverts draft. | Shown for all kinds **except `Text`** (`Group` has its own separate Label field below with identical maxlength/behaviour). Rust: `label` validated `length(max=200)`. SQL: node `label` is bare `TEXT`, no CHECK. |
| Content *(Text only)* | Textarea (4 rows) | Editable | Free-form annotation text, HTML `maxlength=5000`. Committed on blur and synchronized with the existing inline canvas editor without overwriting the field that is actively being edited. | Only when `node.kind === 'Text'`. Controls `text_content`, which is the rendered text body. Rust: `text_content` validated `length(max=5000)`. SQL: bare `TEXT`, no CHECK. |
| Name *(Group only)* | Text input | Editable | Free text, `maxlength=200`, placeholder = i18n `topology_customViewGroupInternalNamePlaceholder` → **"Internal name (not shown on canvas)"**. Same draft/blur/Enter/Escape pattern. | Only when `node.kind === 'Group'`. Rust: `name` validated `length(max=200)`. SQL: bare `TEXT`, no CHECK. |
| Description *(Group only)* | Textarea (2 rows) | Editable | Free text, `maxlength=2000`. Same draft/blur commit pattern (no Escape-revert wired for this one, unlike Label/Name/Badge). | Only when `kind === 'Group'`. Rust: `description` validated `length(max=2000)`. SQL: bare `TEXT`, no CHECK. |
| Show label *(Group only)* | Checkbox | Editable | Boolean, default `true` when unset (`node.show_label ?? true`) | Only when `kind === 'Group'`. SQL column `show_label BOOLEAN NOT NULL DEFAULT TRUE`. |
| Show description *(Group only)* | Checkbox | Editable | Boolean, default `true` when unset | Only when `kind === 'Group'`. SQL: `show_description BOOLEAN NOT NULL DEFAULT TRUE`. |
| Look (radio group: "Image", "Bordered image", "1-2 letter badge", "Stats card") | Radio buttons | Editable | `node.style`: `Image` \| `ImageBordered` \| `Badge` \| `StatsCard`, default `Image` | Only for "object kind" (`kind === 'Entity' \|\| kind === 'Library'`). "Stats card" is itself hidden unless `statsCardAvailable` (`kind === 'Entity' && entity_type === 'Host'`) — Stats-card look is Host-only. SQL: `style TEXT CHECK (style IN ('Image','ImageBordered','Badge','StatsCard'))` — matches. Rust: `style: Option<NodeStyle>` typed enum. |
| Badge text (max 2 chars) | Text input | Editable | Free text, `maxlength=2` (HTML) | Only shown when `style === 'Badge'`. Rust: `badge_text` validated `length(max=2, message="Badge text must be at most 2 characters")` — agrees. SQL: bare `TEXT`, no CHECK. |
| Custom image (Upload image button) | Button/Action + hidden file input | Action | Opens native file picker, `accept="image/*"`; on selection calls `onUploadImage(file)` | Only for object kind (`Entity`/`Library`). No client-side file-size/type enforcement beyond the `accept` hint; server-side limits not verified (outside these files). |
| Show service icon *(Service entity only)* | Checkbox | Editable | Boolean, default `true` when unset. Enables/disables the detected or custom service icon without affecting the label. | Only when `kind === 'Entity' && entity_type === 'Service'`. Detected icons resolve through the same `serviceDefinitions.getIconComponent(service_definition)` metadata helper as L3 Physical; no custom-view icon catalogue exists. |
| Icon position *(Service entity only)* | Dropdown/select | Editable | `BeforeName`, `AfterName`, or `Center` (shown as “Centre of object”); default `BeforeName`. | Before/After participates in the positioned label row. Center replaces the generic centre glyph while label anchoring remains independent. SQL CHECK matches the Rust `ServiceIconPosition` enum. |
| Custom icon URL *(Service entity only)* | URL input + Reset button | Editable/Action | HTTP(S) URL, `maxlength=2048`; an accepted value overrides the detected icon. Reset writes `null` and restores detection. | Rendering applies the shared safe-URL guard; Rust independently rejects non-HTTP(S) schemes and values over 2048 characters. |
| Horizontal *(Service entity only)* | Dropdown/select | Editable | `Left`, `Center` (shown as “Centre”), or `Right`; default `Center`. | Anchors the whole service label/icon row horizontally and combines independently with Vertical. Reuses the existing Rust `TextAlign` enum but persists in `service_label_horizontal_align`, separate from typographic `text_align`. |
| Vertical *(Service entity only)* | Dropdown/select | Editable | `Top`, `Middle`, or `Bottom`; default `Bottom`. | Anchors the service label/icon row vertically and combines independently with Horizontal. SQL CHECK matches `ServiceLabelVerticalAlign`. |
| X offset *(Service entity only)* | Number input | Editable | Integer pixels from -1000 to 1000; default `0`. | Displaces the label/icon row horizontally after anchoring; client, renderer, Rust validation, and SQL CHECK share the same bounds. |
| Y offset *(Service entity only)* | Number input | Editable | Integer pixels from -1000 to 1000; default `0`. | Displaces the label/icon row vertically after anchoring; client, renderer, Rust validation, and SQL CHECK share the same bounds. |
| Corner style (radio: "Rounded", "Square") | Radio buttons | Editable | `node.corner_style`: `Rounded` \| `Square`, default `Rounded` | Shown for all node kinds. SQL: `CHECK (corner_style IN ('Rounded','Square'))` — matches. |
| Font (FontPicker — searchable dropdown) | Custom dropdown/select | Editable | 18 curated fonts from `font-catalog.json` (sans-serif/serif/monospace), plus "System default" (`null`). Search box filters by typing. | `node.font_family`. Rust: `length(max=100)` — free-form string (an earlier migration constrained it to `Sans/Serif/Monospace`; that CHECK was dropped in `20260807050000_custom_view_node_typography_and_group_metadata.sql` so it can hold any curated font id). SQL: bare `TEXT`, no CHECK today. |
| Size (font size, px) | Number input | Editable | `type=number`, HTML `min="10"`, `step="1"`, **no HTML max**. `onchange` additionally requires `Number.isSafeInteger(value) && value >= 10` — no upper bound anywhere in this file. Default displayed `16` if unset. | **Worked example — verified true today:** Rust `font_size: Option<i64>` validated `range(min=10)`, no max. SQL: current constraint (after `20260829113000_remove_font_size_ceiling.sql`) is `CHECK (font_size >= 10)` — floor only. History: `20260730120000_custom_view_text_styling.sql` originally set `CHECK (font_size BETWEEN 10 AND 72)`; that ceiling was dropped 2026-08-29. **All three layers agree: floor 10, no ceiling, node-level.** Contrast with canvas-level default font size (§6.2), which still has a stale ceiling in its Svelte layer only. |
| Bold / Italic / Underline (3-column select group) | 3× dropdown/select | Editable | Tri-state: empty string = "Canvas (On/Off)" (shows the resolved canvas default in the option label), `"true"` = On, `"false"` = Off. Maps to `node.font_bold`/`font_italic`/`font_underline`: `true` \| `false` \| `null` (null = inherit canvas default). | i18n: `common_bold`→"Bold", `common_italic`→"Italic", `common_underline`→"Underline". SQL: originally `BOOLEAN NOT NULL DEFAULT FALSE` (`20260807050000`), changed to nullable in `20260830000000_custom_topology_text_appearance_inheritance.sql` (`DROP NOT NULL`, `DROP DEFAULT`) specifically to support null-means-inherit; old `FALSE` rows backfilled to `NULL` via `NULLIF(font_bold, FALSE)`. Matches current Svelte tri-state. |
| Text align | Dropdown/select | Editable | Options: "Canvas ({resolved default})" (empty string = inherit/null), `Left`, `Center`, `Right`. | `node.text_align`. SQL: `CHECK (text_align IN ('Left','Center','Right'))` when non-null — matches. |
| Border | Dropdown/select | Editable | Fixed list, **no inherit/null option unlike the other style controls**: `None`, `Solid`, `Dashed`, `Dotted`, `Double`. Default `Solid` if unset. | Rust: `border_style: Option<BorderStyle>` typed enum, no `#[validate]`. SQL: `border_style` is bare `TEXT` (added `20260801120000_custom_topology_baseline_styles.sql`) with **no CHECK constraint** — unlike `style`/`corner_style`/`text_align`, which do have DB CHECKs. Relies solely on the Rust enum for validation on the normal API path. |
| Transparency (range slider, label shows `{100 - opacity}%`) | Range slider | Editable | `type=range`, `min=0 max=100`. Displayed label is **inverted** from the stored value: stored `node.opacity` is 0=fully transparent, 100=opaque, but the slider label reads "Transparency (X%)" where X = `100 - opacity`. Default `100` (opaque) if unset. | Rust: `opacity: Option<i64>` validated `range(min=0,max=100)`. SQL: `CHECK (opacity IS NULL OR opacity BETWEEN 0 AND 100)` (added `NOT VALID` in `20260801120000`, validated in `20260801120001`) — all three layers agree numerically; only the display/storage inversion is worth knowing. |
| Link URL | Text input (`type=url`) | Editable | Free text, placeholder "https://…". Committed `onchange`. Empty string coerced to `null`. | Rust: `link_url` validated `length(max=2048)`. SQL: bare `TEXT`, no CHECK. |
| Text colour (swatch grid: "Canvas" + 18 named colors) | Color-swatch buttons | Editable | "Canvas" sets `text_color = null` (inherit); else one of 18 fixed colors (Pink, Rose, Red, Amber, Orange, Green, Emerald, Teal, Cyan, Blue, Indigo, Purple, Fuchsia, Violet, Sky, Gray, Lime, Yellow). | Shown only for `kind` in `Group, Entity, Library, Text`. `text_color` added in `20260830000000`; no DB CHECK restricting it to the 18-color set (Rust `Color` enum constrains it on the normal API path). |
| Primary color (swatch grid, 18 colors, no "inherit") | Color-swatch buttons | Editable | One of the 18 named colors. Clicking sets **both** `primary_color` and legacy `color` to the same value — writing this field keeps the deprecated `color` column in sync. | Shown for `kind` in `Group, Entity, Library, Text`. Both bare `TEXT`, no CHECK. |
| Secondary color (swatch grid, 18 colors) | Color-swatch buttons | Editable | Same 18-color set, no inherit option. | Shown for **every** node kind. Bare `TEXT`, no CHECK. |
| Background color (swatch grid, 18 colors) | Color-swatch buttons | Editable | Same 18-color set, no inherit option. | Shown for every node kind. Bare `TEXT`, no CHECK. |

**Full 18-color palette used throughout §6:** Pink, Rose, Red, Amber, Orange, Green, Emerald, Teal, Cyan, Blue, Indigo, Purple, Fuchsia, Violet, Sky, Gray, Lime, Yellow.

### 6.2 `CanvasControlPanel.svelte`

Floating panel, top-left of canvas. Always shows a collapsed header bar; an expandable body appears when toggled.

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| View name (header, "{view.name}") | Read-only text | Fixed (in header) | Truncated display of the view's current name | Editable via the "Name" field once expanded; this is only the collapsed-state label. |
| Settings toggle (gear/chevron icon button) | Button/Action | Action | Toggles `expanded` state; icon switches `Settings` (collapsed) / `ChevronUp` (expanded) | title = i18n `topology_customViewCanvasSettings` → **"Canvas settings"** |
| Toggle object palette (Blocks icon button) | Button/Action | Action | Calls `onTogglePalette()` | title = i18n `topology_customViewTogglePalette` → **"Toggle object palette"** |
| Delete view (Trash2 icon button, red) | Button/Action | Action | Calls `onDelete()` — deletes the whole custom view | title = i18n `topology_customViewDeleteView` → **"Delete view"** |
| Close (X icon button) | Button/Action | Action | Calls `onClose()` — closes/collapses the panel | title = i18n `common_close` → **"Close"** |
| Name | Text input | Editable | Free text. Committed on blur, **only if non-empty after `.trim()`** — an empty/whitespace-only name silently fails to save. Enter blurs/commits. | Only visible when `expanded`. Rust: `name` validated `length(min=1, max=100, message="Name must be between 1 and 100 characters")`. **Gap**: frontend has **no `maxlength` attribute at all** and no visible error message if the 100-char max is exceeded — it will submit and presumably be rejected server-side with no evident UI feedback path. |
| Description | Textarea (2 rows) | Editable | Free text, **no `maxlength` attribute** (unlike the Group description field in §6.1, which sets `maxlength=2000`). Committed on blur. | Rust: `description` validated `length(max=2000)`. Same client-side gap as Name. SQL: bare `TEXT`, no CHECK. |
| Background colour (swatch grid: "None" + 18 colors) | Color-swatch buttons | Editable | "None" sets `background_color = null`; else one of the 18 named colors. | Bare `TEXT`, no CHECK. |
| Show grid | Checkbox | Editable | Boolean, default `true` when unset | SQL: `show_grid BOOLEAN NOT NULL DEFAULT TRUE`. |
| Snap to grid | Checkbox | Editable | Boolean, default `true` when unset | SQL: `snap_to_grid BOOLEAN NOT NULL DEFAULT TRUE`. |
| Grid size (px) | Number input | Editable | `type=number`, HTML `min="5" max="200" step="1"`. `onchange` re-checks `Number.isInteger(value) && value >= 5 && value <= 200` — HTML and JS bounds agree exactly. Default `20`. | Rust: `grid_size: i64` validated `range(min=5, max=200, message="Grid size must be between 5 and 200 pixels")`. SQL: `CHECK (grid_size BETWEEN 5 AND 200)`, `NOT NULL DEFAULT 20`. **All three layers agree exactly.** |
| Default font (FontPicker) | Custom dropdown/select | Editable | Same 18-font catalog + "System default" as §6.1's Font picker. Label i18n `topology_customViewDefaultFont` → **"Default font"**. | `view.default_font_family`. Rust: `length(max=100)`, free-form. SQL: bare `TEXT`, no CHECK. |
| Default font size | Number input | Editable | `type=number`, HTML `min="10" max="1000" step="1"`, placeholder `16`. **The `onchange` handler (`handleFontSizeChange`) enforces a different, narrower rule**: empty → `null`; otherwise requires `Number.isInteger(value) && value >= 10 && value <= 72` — **values 73–1000 pass the browser's own `max="1000"` validation but are silently dropped by the JS handler and never saved.** | **★ Confirmed three-layer / internal disagreement (the standout finding of this audit):** the HTML `max="1000"` attribute contradicts this same file's own JS ceiling of `72`. Rust `default_font_size: Option<i64>` validates only `range(min=10)` — no ceiling. SQL (post `20260829113000_remove_font_size_ceiling.sql`): `CHECK (default_font_size IS NULL OR default_font_size >= 10)` — floor only. So DB and Rust both accept any value ≥ 10 today, but this one input's JS handler still silently enforces a stale ≤72 ceiling left over from the original `BETWEEN 10 AND 72` constraint (set in `20260807050001_custom_topology_view_canvas_properties.sql` / validated in `...050002`, dropped in `20260829113000`). Net effect: a user typing `100` here sees it accepted by the browser, then silently fails to persist, with no error shown. **The equivalent node-level Size field (§6.1) does not have this bug** — its handler only checks `>= 10`. |
| Default text colour (swatch grid: "Built-in" + 18 colors) | Color-swatch buttons | Editable | "Built-in" sets `default_text_color = null`; else one of 18 named colors. | Added in `20260830000000_custom_topology_text_appearance_inheritance.sql`, bare `TEXT`, no CHECK. |
| Default Bold / Italic / Underline (3-column select group) | 3× dropdown/select | Editable | Tri-state: empty = "Built-in" (`null`), `"true"` = On, `"false"` = Off. The inherit option is labelled "Built-in" (not a resolved value, since there's nothing above canvas-level to inherit from). | `view.default_font_bold/italic/underline`, nullable `BOOLEAN` (added `20260830000000`), no CHECK needed. |
| Default text align | Dropdown/select | Editable | Options: "Built-in (Left)" (empty/null), `Left`, `Center`, `Right`. | SQL: `CHECK (default_text_align IN ('Left','Center','Right'))` added `20260830000000`, validated `20260830000001`. Matches. |
| Default object colour (swatch grid, 18 colors, no inherit) | Color-swatch buttons | Editable | Default `primary_color` assigned to newly created object nodes — not a live override of existing nodes. | `default_primary_color`, bare `TEXT`, no CHECK. |
| Default connector colour (swatch grid, 18 colors, no inherit) | Color-swatch buttons | Editable | Default stroke color for newly created edges. | `default_connector_color`, bare `TEXT`, no CHECK. **Scope note**: there is no per-edge control anywhere in the UI to change an individual edge's connector color after creation (see §6.3) — an edge keeps whatever the canvas default was at creation time; changing this default does not retroactively affect existing edges (`toFlowEdge()` reads `edge.color` directly, not the live canvas default). |

### 6.3 Edge properties — inline "Join settings" panel (in `CustomViewCanvas.svelte`, not a separate component file)

Floating panel, top-right of canvas (same position as the node inspector — the two are mutually exclusive: node selected → §6.1 panel; edge selected → this panel). Title is a **literal, non-i18n string**: "Join settings".

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| Panel title "Join settings" | Read-only text | Fixed | Literal hardcoded string — **not** routed through `$lib/paraglide/messages`, unlike almost everything else in this catalogue. | |
| Connection text | Text input | Editable | Free text; empty string coerced to `null` on save; committed `onchange`. This is the edge's `label`, rendered on-canvas by `CustomViewEdge.svelte`. | Backend: `CustomViewEdgeBase.label` — `#[validate(length(max = 200, message = "Label is too long"))]`. SQL: `custom_topology_view_edges.label` is bare `TEXT`, **no CHECK constraint at all**. Svelte: **no `maxlength` attribute on the input** — a real three-layer gap: Rust caps at 200, Svelte and SQL enforce nothing. |
| Dependency (checkbox) | Checkbox | Editable | Boolean `is_dependency`, default `false`. Also drives on-canvas rendering: `animated: edge.is_dependency` plus a dashed stroke (`stroke-dasharray: 6 4`) appended when true. | SQL: `is_dependency BOOLEAN NOT NULL DEFAULT FALSE` (added in `20260801120000_custom_topology_baseline_styles.sql`). No Rust `#[validate]` needed (plain bool). |
| Link URL | Text input (`type=url`) | Editable | Free text, placeholder "https://…", empty → `null`, committed on `onchange`. | Backend: `CustomViewEdgeBase.link_url` — `#[validate(length(max = 2048, message = "Link is too long"))]`. SQL: bare `TEXT` (added in `20260801120000`), no CHECK. Svelte: no `maxlength` — same gap pattern as Connection text. |
| Open link (button, conditional) | Button/Action | Action | Opens `selectedEdge.link_url` in a new tab (`window.open(..., '_blank', 'noopener,noreferrer')`) | Only rendered when `getSafeCanvasLink(selectedEdge.link_url)` returns truthy (a scheme/safety check on the URL; not further verified). |
| Delete edge (button) | Button/Action | Action | Deletes the selected edge | Plain hardcoded label "Delete edge", not i18n-routed. |

**Not editable per-edge at all:** font family, font size, text color, bold/italic/underline, text align. `CustomViewEdge.svelte` (the renderer) sources every one of these purely from the canvas-wide `default_font_*`/`default_text_color`/`default_text_align` settings (`toFlowEdge()` hardcodes `data: { fontFamily: currentView?.default_font_family ?? null, fontSize: currentView?.default_font_size ?? 16, ... }` with no per-edge override field anywhere). A real, current asymmetry with nodes (which can override every one of these individually) — documented as current behavior, not assessed as a defect.

**Backend model note:** `CustomViewEdgeBase` also declares `source_handle`/`target_handle` (`length(max = 40)`, which side of each node the edge was dragged from/to — used for re-rendering only, not exposed in this panel) and a `color: Option<Color>` field — the backend supports a per-edge color, but **no control in the Join settings panel sets it**; an edge's rendered color comes solely from the canvas-wide `default_connector_color` at creation time (§6.2).

### 6.4 `FontPicker.svelte`

Reusable searchable dropdown, used by both §6.1 and §6.2 (not a standalone panel).

| Property / Item | Control Type | Editable / Fixed / Action | Available Values / Behaviour | Conditions / Notes |
|---|---|---|---|---|
| Trigger button (shows current font name or "System default", rendered in that font) | Button/Action | Action (opens dropdown) | Displays `value ?? 'System default'` | The literal string "System default" is **hardcoded, not i18n-routed**. |
| Search box | Text input | N/A (filter only, not persisted) | Placeholder = i18n `topology_customViewFontSearchPlaceholder` → **"Search fonts…"**. Filters the 18-entry catalog client-side. | |
| "System default" option (always first) | Button/Action (list item) | Editable (sets the bound field) | Selecting calls `onSelect(null)` | Hardcoded string, not i18n. |
| Font list (up to 18 entries from `font-catalog.json`) | Button/Action (list item) per font | Editable (sets the bound field) | Each previewed live in its own typeface (lazy-loaded while the picker is open); selecting calls `onSelect(font.id)` | "No fonts match "{search}"" empty-state text is also hardcoded, not i18n. |

`font-catalog.json` contains exactly **18** font entries (`id`, `slug`, `category`, and a weight→woff2 URL map, weights `400`/`700` observed) — the complete, exhaustive font set offered anywhere on the custom canvas; the same list backs both the per-node and canvas-level font pickers.

### 6.5 Custom canvas — notes

- Node position/size fields (`x`, `y`, `width`, `height`) exist in the schema (SQL: `x`/`y` `CHECK (... BETWEEN -1000000 AND 1000000)`; Rust: plain `i64` with **no `#[validate(range)]` attribute at all** — a Rust/SQL gap) but are set by dragging/resizing on the canvas, not through any input field in either properties panel — out of scope for a properties-panel catalogue proper, noted here only for completeness.
- `CustomGroupNode.svelte`, `CustomObjectNode.svelte`, `CustomTextNode.svelte`, `CustomViewPalette.svelte` (the drag-source stencil palette) were not catalogued — they are canvas renderers/pickers, not properties panels, per the task's explicit file scope.

---

## 7. Differences Between Entity Types

**Tag-picker coverage** (which entities can have tags added/removed from any inspector panel):
- **Can**: Host, Service, Subnet (via `TagPickerInline`, gated on `editState.isEditable` + `manage_org_entities` permission for creation).
- **Cannot**: IPAddress, Interface (SNMP ifEntry), Dependency — their Display configs have no `getTagPickerProps`, and their `getTags` don't even surface the entity's own `tags` array (only derived/synthetic tags like subnet CIDR or oper-status).
- Edge-card variants follow the entity being displayed: 6 of 8 edge-kind panels embed a tag picker on their Host/Service/Subnet cards (§3.1); `InspectorEdgeNeighborLink`/`InspectorEdgePhysicalLink` do not, because their cards are Host/Interface, and Interface has no tag picker.

**Inline-editable description coverage**:
- **Can**: Host (§2.3), Subnet (§2.7) — both via `InlineDescription`, both 500-char Svelte/Rust limit.
- **Cannot**: Service — no `showEditableEntityDescription`/`onEntityDescriptionSave` wiring exists on `ServiceDisplayContext` at all.
- **Dependency** has its own distinct inline-editable description (§3.4), separate from the Host/Subnet pattern (500-char Rust cap, no Svelte maxlength, no SQL CHECK).

**Per-view section sets differ substantially** (§1.1): L2Physical shows the fewest element fields (Identity + raw SNMP interface data only — no Services, no HostDetail, no OtherInterfaces); L3Logical and Workloads show the richest element view (Identity, host/virtualization context, Services, sibling IPs); Application view swaps host-centric sections for Dependencies + Application-tag sections and is the only view with `show_application_picker: true`. Container sections likewise range from a single "Subnet detail" card (L3) to Identity+ElementSummary+DependencySummary (Application).

**Custom-canvas nodes vs. discovery-driven nodes** are entirely separate object models with no field overlap: custom-view nodes (§6) have their own typography/color/border/opacity system with inheritance from canvas-level defaults; discovery-driven nodes (§2) have no styling properties at all — every field there is either identity/context data or a tag.

**Custom-canvas node kinds differ from each other** (§6.1): `Text` nodes have no Label field (edited inline via `contenteditable`, not the inspector) and no "Look"/Badge/Corner-style/Upload-image controls; `Group` nodes uniquely get Name, Description, Show label, Show description; `Entity`/`Library` nodes uniquely get the Look radio group (with Stats-card gated further to Host-typed Entity nodes only) and file upload. Secondary/Background color and Corner style are the only style fields shown for **every** kind including `Text`.

**Custom-canvas edges vs. nodes**: edges expose far fewer properties (Connection text, Dependency flag, Link URL — §6.3) and have zero per-edge styling — font, color, and text alignment are always inherited from the canvas-wide defaults, whereas every one of those is individually overridable on a node.

---

## 8. Cross-Layer Validation Disagreements

Every genuine three-layer (or internal-Svelte) disagreement found during this audit, in one place:

| Field | Svelte (client) | Rust API validator | SQL CHECK | Disagreement |
|---|---|---|---|---|
| Custom-view **node** `font_size` (§6.1) | `min=10`, no max | `range(min=10)`, no max | `CHECK (font_size >= 10)` (ceiling of 72 removed in `20260829113000_remove_font_size_ceiling.sql`) | **None** — all three agree today. Documented as the confirmed baseline the audit's worked example describes. |
| Custom-view **canvas-level** `default_font_size` (§6.2) | HTML `max="1000"`, but the `onchange` handler additionally enforces `<= 72` | `range(min=10)`, no max | `CHECK (default_font_size IS NULL OR default_font_size >= 10)`, no ceiling | **Yes — the standout finding.** The Svelte HTML attribute (max 1000) contradicts the same file's own JS ceiling (72); neither matches Rust/SQL, which impose no ceiling at all. Values 73–1000 pass browser validation, then silently fail to save with no visible error. |
| Dependency `name` (§4.5) | `required` (non-empty) + `max(100)` | `length(min = 0, max = 100)` — empty allowed | `dependencies.name TEXT NOT NULL` — no length CHECK, NOT NULL doesn't block `''` | **Yes.** Frontend blocks an empty name; backend and DB both accept one. |
| Tag `name` (§2.1) | No client-side max length or charset check | `length(min = 1, max = 100)` | No CHECK found on `tags.name` | **Yes.** Svelte will let a user type/submit an arbitrarily long name that the API will then reject (min=1 also unenforced client-side beyond a bare non-empty trim check). |
| Custom-view **edge** `label` (§6.3) | No `maxlength` attribute | `length(max = 200, message = "Label is too long")` | Bare `TEXT`, no CHECK | **Yes** — and notably inconsistent with the **node** `label` field, which does set a matching Svelte `maxlength=200`. |
| Custom-view **edge** `link_url` (§6.3) | No `maxlength` attribute | `length(max = 2048, message = "Link is too long")` | Bare `TEXT`, no CHECK | **Yes** — same pattern; the **node** `link_url` field has no Svelte maxlength either, so this one is consistent with its node counterpart but still ungoverned client-side. |
| Custom-view **canvas** `name` (§6.2) | No `maxlength`; only "don't save if blank after trim" logic | `length(min = 1, max = 100)` | Bare `TEXT`, no CHECK | **Partial.** Frontend approximates the `min=1` rule (via the blank-save-skip) but enforces no maximum and shows no error if 100 is exceeded. |
| Custom-view **canvas** `description` (§6.2) | No `maxlength` (contrast: the Group-node description field, §6.1, does set `maxlength=2000`) | `length(max = 2000)` | Bare `TEXT`, no CHECK | **Yes**, by omission — same backend limit as the Group node description, but only one of the two Svelte inputs mirrors it. |
| `border_style` on custom-view nodes (§6.1) | Fixed 5-option dropdown (`None`/`Solid`/`Dashed`/`Dotted`/`Double`) | `Option<BorderStyle>` typed enum, no explicit `#[validate]` | **No CHECK constraint** — unlike sibling style columns `style`, `corner_style`, `text_align`, which all have one | Not a live disagreement (the Rust enum still rejects bad values through the normal API), but a **defense-in-depth gap**: a direct DB write could set an arbitrary string, unlike the other style enums. |

Fields checked and found to **agree** (or to have only a silently-unenforced third layer, not a conflicting one) are noted in place rather than repeated here: `grid_size` (§6.2, all three layers match exactly, 5–200), node `font_size` (above), `opacity` (§6.1, 0–100 on all three, only the display-vs-storage inversion differs), `text_align`/`corner_style`/`style` (all three layers agree via CHECK+enum), Host/Subnet `description` (§2.3/§2.7, Svelte/Rust agree at 500, SQL simply has no CHECK), Dependency `description` (§3.4, Rust caps at 500, no SQL CHECK, no Svelte maxlength found — silent rather than conflicting since nothing permits a value the others would reject).

---

## 9. Unknown / Could Not Determine

Stated plainly rather than guessed:

- **Whether the missing entity tag picker on `InspectorEdgeNeighborLink`/`InspectorEdgePhysicalLink` host cards (§3.1, §3.7, §3.8) is intentional** — recorded only as an observed difference from the other 6 edge-kind panels, not investigated further (e.g. against product intent or an issue tracker).
- **Server-side tag-name dedupe/collision behaviour**: `TagPickerInline`'s "exact match" check only compares against tags already loaded client-side (`tagsQuery`/`availableTagsProp`, a potentially stale/partial list) — whether the backend enforces a uniqueness constraint (e.g. a case-insensitive unique index on `(organization_id, name)`) was not verified in `backend/src/server/tags/impl/base.rs` or its migrations.
- **Random tag-color assignment**: whether two tags can be assigned the same random color (from `AVAILABLE_COLORS`) with no collision-avoidance was not verified.
- **Custom-canvas image upload**: no client-side file-size/type validation was found beyond the `accept="image/*"` picker hint (§6.1); server-side limits on `host_images`/upload handling were not checked (out of this task's file scope).
- **`getSafeCanvasLink()`** (gates the "Open link" button in §3.4/§6.3): the exact URL-scheme/safety check it performs was not traced — assumed to block unsafe schemes (e.g. `javascript:`) but not confirmed by reading its implementation.
- **User-visible error surfacing on server-side validation rejection**: e.g. what a user sees if they exceed the canvas-view name's 100-character backend limit (§6.2, no Svelte maxlength there) or otherwise trigger a Rust `#[validate]` failure — no toast/error-handling code was found in any of the files read for this audit. It is unknown whether such saves fail silently, show a generic error, or surface the specific validation message.
- **Whether a dedicated Tags administration panel exists elsewhere in the app** (outside `ui/src/lib/features/topology`) with its own properties/validation for tag creation/editing beyond the inline `TagPickerInline` covered here — `ui/src/lib/features/tags/components/` (`TagPicker.svelte`, `TagEditModal.svelte`, `TagTab.svelte`) clearly exists but was not catalogued, since it is not a topology/custom-canvas/share Properties Panel per the task's file scope.
- **`OptionsContent.svelte`** (the Filters / Groups / Display tabs shown by `TopologyOptionsPanel` when nothing is selected, §1) was not catalogued at all — it is a view/display-options configuration surface, not an entity Properties Panel, and was judged out of scope; if a future audit wants it included, it is a distinct component tree not touched here.
- **Whether `CustomViewPalette.svelte`** (the drag-source object palette on the custom canvas) has any configurable properties of its own, as opposed to being a static picker of stencils to drag onto the canvas, was not determined — it was not read in full.

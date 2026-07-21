-- User-authored custom topology views: unlike the built-in L2/L3/Workloads/
-- Application views (computed live from entity data every request), a custom
-- view's nodes and edges are hand-placed by the user and persisted as-is.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

CREATE TABLE custom_topology_views (
    id UUID PRIMARY KEY,
    network_id UUID NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_custom_topology_views_network_id ON custom_topology_views(network_id);

-- Reusable catalog of "common object" stencils (router, switch, firewall,
-- cloud, etc.) for the custom-view palette. `organization_id IS NULL` rows are
-- the seeded built-in catalog (protected from update/delete at the service
-- layer); non-null rows are an organization's own additions.
CREATE TABLE library_objects (
    id UUID PRIMARY KEY,
    organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    -- Kebab-case lucide icon name (e.g. "router"), used when no uploaded
    -- image is set.
    icon TEXT,
    color TEXT,
    storage_path TEXT,
    content_type TEXT,
    size_bytes BIGINT,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_library_objects_organization_id ON library_objects(organization_id);

INSERT INTO library_objects (id, organization_id, name, icon, color, created_at, updated_at) VALUES
    (gen_random_uuid(), NULL, 'Router', 'router', 'Blue', now(), now()),
    (gen_random_uuid(), NULL, 'Switch', 'network', 'Indigo', now(), now()),
    (gen_random_uuid(), NULL, 'Firewall', 'shield', 'Red', now(), now()),
    (gen_random_uuid(), NULL, 'Load Balancer', 'waypoints', 'Purple', now(), now()),
    (gen_random_uuid(), NULL, 'Cloud / Internet', 'cloud', 'Sky', now(), now()),
    (gen_random_uuid(), NULL, 'Server', 'server', 'Amber', now(), now()),
    (gen_random_uuid(), NULL, 'Database', 'database', 'Emerald', now(), now()),
    (gen_random_uuid(), NULL, 'Desktop', 'monitor', 'Teal', now(), now()),
    (gen_random_uuid(), NULL, 'Laptop', 'laptop', 'Teal', now(), now()),
    (gen_random_uuid(), NULL, 'Printer', 'printer', 'Gray', now(), now()),
    (gen_random_uuid(), NULL, 'Wireless AP', 'wifi', 'Cyan', now(), now()),
    (gen_random_uuid(), NULL, 'Camera', 'cctv', 'Gray', now(), now()),
    (gen_random_uuid(), NULL, 'UPS', 'battery', 'Orange', now(), now()),
    (gen_random_uuid(), NULL, 'NAS / Storage', 'hard-drive', 'Violet', now(), now()),
    (gen_random_uuid(), NULL, 'VPN Gateway', 'globe-lock', 'Rose', now(), now()),
    (gen_random_uuid(), NULL, 'Modem', 'radio-tower', 'Lime', now(), now());

-- Nodes placed on a custom view's canvas. `kind` determines which of the
-- other columns are meaningful:
--   entity  -> entity_id + entity_type reference a real inventory entity
--   library -> library_object_id references a library_objects stencil
--   text    -> text_content is the freeform annotation body
--   group   -> a colored named frame; color/corner_style/width/height apply,
--              and other nodes reference it via parent_node_id
CREATE TABLE custom_topology_view_nodes (
    id UUID PRIMARY KEY,
    view_id UUID NOT NULL REFERENCES custom_topology_views(id) ON DELETE CASCADE,
    -- Denormalized from custom_topology_views.network_id, same pattern as
    -- host_images.network_id, so the generic child-query network-scoping
    -- filter works without a join.
    network_id UUID NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('Entity', 'Library', 'Text', 'Group')),
    entity_id UUID,
    entity_type TEXT,
    library_object_id UUID REFERENCES library_objects(id) ON DELETE SET NULL,
    text_content TEXT,
    label TEXT,
    style TEXT CHECK (style IN ('Image', 'ImageBordered', 'Badge', 'StatsCard')),
    badge_text TEXT,
    color TEXT,
    corner_style TEXT CHECK (corner_style IN ('Rounded', 'Square')),
    parent_node_id UUID REFERENCES custom_topology_view_nodes(id) ON DELETE SET NULL,
    x BIGINT NOT NULL CHECK (x BETWEEN -1000000 AND 1000000),
    y BIGINT NOT NULL CHECK (y BETWEEN -1000000 AND 1000000),
    width BIGINT,
    height BIGINT,
    storage_path TEXT,
    content_type TEXT,
    size_bytes BIGINT,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_custom_topology_view_nodes_view_id ON custom_topology_view_nodes(view_id);
CREATE INDEX idx_custom_topology_view_nodes_network_id ON custom_topology_view_nodes(network_id);
CREATE INDEX idx_custom_topology_view_nodes_parent_node_id ON custom_topology_view_nodes(parent_node_id);

-- Manually drawn edges between two nodes on the same custom view.
CREATE TABLE custom_topology_view_edges (
    id UUID PRIMARY KEY,
    view_id UUID NOT NULL REFERENCES custom_topology_views(id) ON DELETE CASCADE,
    network_id UUID NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    source_node_id UUID NOT NULL REFERENCES custom_topology_view_nodes(id) ON DELETE CASCADE,
    target_node_id UUID NOT NULL REFERENCES custom_topology_view_nodes(id) ON DELETE CASCADE,
    label TEXT,
    color TEXT,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_custom_topology_view_edges_view_id ON custom_topology_view_edges(view_id);
CREATE INDEX idx_custom_topology_view_edges_network_id ON custom_topology_view_edges(network_id);

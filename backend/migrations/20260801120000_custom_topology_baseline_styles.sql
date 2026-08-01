-- Give every custom-canvas object the same baseline appearance, sizing, and linking controls.
ALTER TABLE custom_topology_view_nodes
    ADD COLUMN font_style TEXT,
    ADD COLUMN primary_color TEXT,
    ADD COLUMN secondary_color TEXT,
    ADD COLUMN background_color TEXT,
    ADD COLUMN opacity BIGINT,
    ADD COLUMN border_style TEXT,
    ADD COLUMN link_url TEXT,
    ADD CONSTRAINT custom_topology_view_nodes_opacity_check
        CHECK (opacity IS NULL OR opacity BETWEEN 0 AND 100);

-- Existing color values remain readable by older releases and seed the new primary color.
UPDATE custom_topology_view_nodes SET primary_color = color WHERE primary_color IS NULL;

ALTER TABLE custom_topology_view_edges
    ADD COLUMN is_dependency BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN link_url TEXT;

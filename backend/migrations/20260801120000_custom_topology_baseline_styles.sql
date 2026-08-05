-- Give every custom-canvas object the same baseline appearance, sizing, and linking controls.
--
-- ADD CONSTRAINT ... NOT VALID here; VALIDATE in the next migration, following the
-- expand-and-contract pattern from the project migration guidelines. `opacity` is a
-- brand-new column (NULL on every existing row until a later write sets it), so the
-- check is trivially satisfied today — NOT VALID only avoids the table-scan lock the
-- project linter requires regardless of how cheap that scan would be in practice.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE custom_topology_view_nodes
    ADD COLUMN font_style TEXT,
    ADD COLUMN primary_color TEXT,
    ADD COLUMN secondary_color TEXT,
    ADD COLUMN background_color TEXT,
    ADD COLUMN opacity BIGINT,
    ADD COLUMN border_style TEXT,
    ADD COLUMN link_url TEXT,
    ADD CONSTRAINT custom_topology_view_nodes_opacity_check
        CHECK (opacity IS NULL OR opacity BETWEEN 0 AND 100) NOT VALID;

-- Existing color values remain readable by older releases and seed the new primary color.
UPDATE custom_topology_view_nodes SET primary_color = color WHERE primary_color IS NULL;

ALTER TABLE custom_topology_view_edges
    ADD COLUMN is_dependency BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN link_url TEXT;

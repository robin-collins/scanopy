-- Editable canvas-level properties for the custom topology view control
-- panel: description, background colour, grid display/snap, and canvas-wide
-- defaults for font and object/connector styling that per-node/edge settings
-- override.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE custom_topology_views
    ADD COLUMN description TEXT,
    ADD COLUMN background_color TEXT,
    ADD COLUMN show_grid BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN grid_size BIGINT NOT NULL DEFAULT 20,
    ADD COLUMN snap_to_grid BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN default_font_family TEXT,
    ADD COLUMN default_font_size BIGINT,
    ADD COLUMN default_primary_color TEXT,
    ADD COLUMN default_connector_color TEXT,
    ADD CONSTRAINT custom_topology_views_grid_size_check
        CHECK (grid_size BETWEEN 5 AND 200) NOT VALID,
    ADD CONSTRAINT custom_topology_views_default_font_size_check
        CHECK (default_font_size IS NULL OR default_font_size BETWEEN 10 AND 72) NOT VALID;

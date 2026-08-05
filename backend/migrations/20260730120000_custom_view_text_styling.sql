-- Persist typography controls for custom-view text annotations. Nullable
-- columns keep existing nodes and mixed-version deployments compatible.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE custom_topology_view_nodes
    ADD COLUMN font_family TEXT
        CHECK (font_family IN ('Sans', 'Serif', 'Monospace')),
    ADD COLUMN font_size BIGINT
        CHECK (font_size BETWEEN 10 AND 72);

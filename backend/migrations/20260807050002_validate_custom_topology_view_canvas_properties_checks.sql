-- Contract half of the expand-and-contract pattern started in
-- 20260807050001_custom_topology_view_canvas_properties.sql.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE custom_topology_views
    VALIDATE CONSTRAINT custom_topology_views_grid_size_check;

ALTER TABLE custom_topology_views
    VALIDATE CONSTRAINT custom_topology_views_default_font_size_check;

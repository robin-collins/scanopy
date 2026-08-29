-- Migration to remove the 72px font-size ceiling for custom topology views.
-- This migration drops existing CHECK constraints and replaces them with a floor-only constraint (10px minimum).
-- This replaces constraints from:
--   20260807050001_custom_topology_view_canvas_properties.sql
--   20260730120000_custom_view_text_styling.sql

BEGIN;

-- 1. Update custom_topology_views
-- Drop the old constraint first.
ALTER TABLE custom_topology_views
    DROP CONSTRAINT custom_topology_views_default_font_size_check;

-- Add the new constraint (floor only).
ALTER TABLE custom_topology_views
    ADD CONSTRAINT custom_topology_views_default_font_size_check
        CHECK (default_font_size IS NULL OR default_font_size >= 10);

-- 2. Update custom_topology_view_nodes
-- Drop the old constraint first.
ALTER TABLE custom_topology_view_nodes
    DROP CONSTRAINT custom_topology_view_nodes_font_size_check;

-- Add the new constraint (floor only).
ALTER TABLE custom_topology_view_nodes
    ADD CONSTRAINT custom_topology_view_nodes_font_size_check
        CHECK (font_size >= 10);

COMMIT;

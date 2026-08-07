-- Group frame metadata (name distinct from label, description, visibility
-- toggles) and canvas-wide typography: replace the coarse font_style enum
-- (Normal/Bold/Italic/BoldItalic) with discrete bold/italic/underline
-- toggles, and add text alignment. Also frees font_family from the fixed
-- Sans/Serif/Monospace enum so it can hold any curated Google Font id.
--
-- font_style is superseded by font_bold/font_italic/font_underline below but
-- deliberately NOT dropped here — expand-and-contract for column removal
-- (squawk's ban-drop-column rule), drop it in a later migration once no
-- deployed server version still reads it.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE custom_topology_view_nodes
    DROP CONSTRAINT IF EXISTS custom_topology_view_nodes_font_family_check;

ALTER TABLE custom_topology_view_nodes
    ADD COLUMN font_bold BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN font_italic BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN font_underline BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN text_align TEXT CHECK (text_align IN ('Left', 'Center', 'Right')),
    ADD COLUMN name TEXT,
    ADD COLUMN description TEXT,
    ADD COLUMN show_label BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN show_description BOOLEAN NOT NULL DEFAULT TRUE;

UPDATE custom_topology_view_nodes
    SET font_bold = TRUE
    WHERE font_style IN ('Bold', 'BoldItalic');

UPDATE custom_topology_view_nodes
    SET font_italic = TRUE
    WHERE font_style IN ('Italic', 'BoldItalic');

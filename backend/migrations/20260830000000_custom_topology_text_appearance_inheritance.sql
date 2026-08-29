-- Separate text colour from decorative colour and make every remaining text
-- appearance property inheritable. Existing FALSE emphasis values become
-- NULL: both render as off against the built-in defaults, but NULL can follow
-- a canvas default when the user later changes it. Existing TRUE values remain
-- explicit overrides.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE custom_topology_view_nodes
    ADD COLUMN text_color TEXT,
    ALTER COLUMN font_bold DROP DEFAULT,
    ALTER COLUMN font_bold DROP NOT NULL,
    ALTER COLUMN font_italic DROP DEFAULT,
    ALTER COLUMN font_italic DROP NOT NULL,
    ALTER COLUMN font_underline DROP DEFAULT,
    ALTER COLUMN font_underline DROP NOT NULL;

UPDATE custom_topology_view_nodes
    SET font_bold = NULLIF(font_bold, FALSE),
        font_italic = NULLIF(font_italic, FALSE),
        font_underline = NULLIF(font_underline, FALSE);

ALTER TABLE custom_topology_views
    ADD COLUMN default_text_color TEXT,
    ADD COLUMN default_font_bold BOOLEAN,
    ADD COLUMN default_font_italic BOOLEAN,
    ADD COLUMN default_font_underline BOOLEAN,
    ADD COLUMN default_text_align TEXT
        CHECK (default_text_align IN ('Left', 'Center', 'Right'));

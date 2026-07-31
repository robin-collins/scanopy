-- New nullable column, defaulting NULL for every existing row, so the FK's
-- validation scan is trivially fast — squawk's adding-foreign-key-constraint
-- rule is suppressed for this file in lint-migrations.sh, same treatment as
-- 20260719140000_host_images.sql's topology_icon_image_id.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE hosts ADD COLUMN category_id UUID REFERENCES categories(id) ON DELETE SET NULL;

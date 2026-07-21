-- Host image gallery: uploaded image files, stored on a filesystem volume
-- (storage_path is relative to the server's configured data directory, not
-- in this table) with metadata only in Postgres.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

CREATE TABLE host_images (
    id UUID PRIMARY KEY,
    host_id UUID NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    -- Denormalized from hosts.network_id, same as ports/interfaces/ip_addresses,
    -- so the generic HostChildQuery network-scoping filter (multi-tenant
    -- isolation) works without a join.
    network_id UUID NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    filename TEXT NOT NULL,
    content_type TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    storage_path TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_host_images_host_id ON host_images(host_id);
CREATE INDEX idx_host_images_network_id ON host_images(network_id);

-- Which of a host's images (if any) to render as its topology node icon.
-- SET NULL on image delete so removing an image never leaves a dangling
-- reference; the host simply falls back to the default node shape.
-- New nullable column, defaulting NULL for every existing row, so the FK's
-- validation scan is trivially fast — squawk's adding-foreign-key-constraint
-- rule is suppressed for this file in lint-migrations.sh.
ALTER TABLE hosts
    ADD COLUMN topology_icon_image_id UUID REFERENCES host_images(id) ON DELETE SET NULL;

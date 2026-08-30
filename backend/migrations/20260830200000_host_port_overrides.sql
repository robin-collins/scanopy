-- Per-host display overrides for ports (well-known and unclaimed alike).
--
-- Keyed on the value tuple (host_id, port_number, port_protocol) rather than the
-- port row UUID because rescans recreate port rows; a UUID FK would silently orphan
-- every override the user had set. service_ref_* are the SHAPE of the catalogue
-- reference from issues #11/#12 (a tagged union: discriminator kind + id) and are
-- inert when NULL (meaning 'not assigned'); they carry no FK and are validated at
-- the API. The delete-blocking rule for referenced custom catalogue entries is
-- enforced by the API (invariant 5), not by the DB.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

CREATE TABLE host_port_overrides (
    id UUID PRIMARY KEY,
    host_id UUID NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    -- Denormalized from the host's network_id, like host_images: required for the
    -- generic network-scoped access-control filter.
    network_id UUID NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    port_number INTEGER NOT NULL,
    port_protocol TEXT NOT NULL,
    display_name TEXT,
    icon_url TEXT,
    service_ref_kind TEXT,
    service_ref_id TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT host_port_overrides_unique_host_port UNIQUE (host_id, port_number, port_protocol),
    CONSTRAINT host_port_overrides_port_number_check
        CHECK (port_number BETWEEN 0 AND 65535) NOT VALID,
    CONSTRAINT host_port_overrides_port_protocol_check
        CHECK (port_protocol IN ('Tcp', 'Udp')) NOT VALID,
    CONSTRAINT host_port_overrides_service_ref_kind_check
        CHECK (service_ref_kind IS NULL OR service_ref_kind IN ('BuiltIn', 'Custom')) NOT VALID,
    CONSTRAINT host_port_overrides_service_ref_pairing_check
        CHECK ((service_ref_kind IS NULL) = (service_ref_id IS NULL)) NOT VALID,
    CONSTRAINT host_port_overrides_display_name_length_check
        CHECK (display_name IS NULL OR char_length(display_name) <= 255) NOT VALID,
    CONSTRAINT host_port_overrides_icon_url_length_check
        CHECK (icon_url IS NULL OR char_length(icon_url) <= 2048) NOT VALID
);

CREATE INDEX idx_host_port_overrides_host ON host_port_overrides(host_id);

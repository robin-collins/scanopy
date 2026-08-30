-- Organization-owned extensions to the compile-time PortType catalogue.
-- Built-ins deliberately remain in Rust and are merged with these rows by the
-- backend; this table can therefore never mutate or masquerade as a built-in.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

CREATE TABLE custom_known_ports (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    port_number BIGINT NOT NULL,
    transport_protocol TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT custom_known_ports_name_length_check
        CHECK (char_length(btrim(name)) BETWEEN 1 AND 100) NOT VALID,
    CONSTRAINT custom_known_ports_description_length_check
        CHECK (description IS NULL OR char_length(description) <= 500) NOT VALID,
    CONSTRAINT custom_known_ports_number_check
        CHECK (port_number BETWEEN 1 AND 65535) NOT VALID,
    CONSTRAINT custom_known_ports_protocol_check
        CHECK (transport_protocol IN ('Tcp', 'Udp')) NOT VALID
);

CREATE INDEX idx_custom_known_ports_organization_id
    ON custom_known_ports(organization_id);
CREATE UNIQUE INDEX idx_custom_known_ports_endpoint
    ON custom_known_ports(organization_id, port_number, transport_protocol);

-- Validate separately so the schema-expansion migration does not take a long
-- access-exclusive lock while checking existing rows.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE host_port_overrides
    VALIDATE CONSTRAINT host_port_overrides_port_number_check,
    VALIDATE CONSTRAINT host_port_overrides_port_protocol_check,
    VALIDATE CONSTRAINT host_port_overrides_service_ref_kind_check,
    VALIDATE CONSTRAINT host_port_overrides_service_ref_pairing_check,
    VALIDATE CONSTRAINT host_port_overrides_display_name_length_check,
    VALIDATE CONSTRAINT host_port_overrides_icon_url_length_check;

-- Validate separately so the schema-expansion migration never holds a long
-- access-exclusive lock while scanning an existing table.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE custom_known_ports
    VALIDATE CONSTRAINT custom_known_ports_name_length_check,
    VALIDATE CONSTRAINT custom_known_ports_description_length_check,
    VALIDATE CONSTRAINT custom_known_ports_number_check,
    VALIDATE CONSTRAINT custom_known_ports_protocol_check;

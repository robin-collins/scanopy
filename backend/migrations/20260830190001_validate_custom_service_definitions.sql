-- Validate separately so the schema-expansion migration does not take a long
-- access-exclusive lock while checking existing rows.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE custom_service_definitions
    VALIDATE CONSTRAINT custom_service_definitions_name_length_check,
    VALIDATE CONSTRAINT custom_service_definitions_description_length_check,
    VALIDATE CONSTRAINT custom_service_definitions_logo_url_length_check,
    VALIDATE CONSTRAINT custom_service_definitions_category_present_check;
-- Global custom service definitions: user-created entries that extend the
-- compile-time built-in service catalogue (ServiceDefinitionRegistry).
-- Built-in definitions have no rows here (they are Rust types), so "built-in
-- is read-only" is automatic; the backend API rejects any custom name that
-- would collide (case-insensitively) with a built-in id, and the uniqueness
-- index below keeps custom names disjoint from each other the same way.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

CREATE TABLE custom_service_definitions (
    id UUID PRIMARY KEY,
    -- Tenant scoping: every custom service belongs to exactly one organization.
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    -- Service id shown in pickers and stored in services.service_definition.
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    -- ServiceCategory id (Rust enum; no FK, matching how categories exist).
    category TEXT NOT NULL,
    logo_url TEXT NOT NULL DEFAULT '',
    logo_needs_white_background BOOLEAN NOT NULL DEFAULT FALSE,
    is_generic BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT custom_service_definitions_name_length_check
        CHECK (char_length(name) BETWEEN 1 AND 39) NOT VALID,
    CONSTRAINT custom_service_definitions_description_length_check
        CHECK (char_length(description) <= 100) NOT VALID,
    CONSTRAINT custom_service_definitions_logo_url_length_check
        CHECK (char_length(logo_url) <= 2048) NOT VALID,
    CONSTRAINT custom_service_definitions_category_present_check
        CHECK (char_length(category) > 0) NOT VALID
);

-- Case-insensitive uniqueness PER ORGANIZATION, matching the API collision
-- check against built-in ids (both are compared case-insensitively). A custom
-- name is scoped to its org: two orgs may each define their own "Internal API",
-- but one org cannot hold two case-variants of the same name.
CREATE UNIQUE INDEX idx_custom_service_definitions_org_name
    ON custom_service_definitions (organization_id, lower(name));

CREATE INDEX idx_custom_service_definitions_organization
    ON custom_service_definitions (organization_id);
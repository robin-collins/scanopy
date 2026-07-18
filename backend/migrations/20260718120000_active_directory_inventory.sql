-- First-class, bounded Active Directory inventory storage.
--
-- This intentionally stores normalized inventory/topology attributes only. It
-- does not provide a JSON/raw-LDAP escape hatch, so credentials, users, LAPS
-- data, arbitrary directory attributes, and attack-path data cannot be
-- persisted accidentally. Collection replacement is scoped by
-- (network_id, collection_key) in the application transaction.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

CREATE TABLE ad_collection_runs (
    id UUID PRIMARY KEY,
    received_order BIGINT GENERATED ALWAYS AS IDENTITY UNIQUE,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    network_id UUID NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    daemon_id UUID NULL REFERENCES daemons(id) ON DELETE SET NULL,
    credential_id UUID NULL REFERENCES credentials(id) ON DELETE SET NULL,
    target_host_id UUID NULL REFERENCES hosts(id) ON DELETE SET NULL,
    target_ip INET NOT NULL,
    discovery_id UUID NULL REFERENCES discovery(id) ON DELETE SET NULL,
    session_id UUID NOT NULL,
    collection_key TEXT NOT NULL CHECK (char_length(collection_key) BETWEEN 1 AND 200),
    collector TEXT NOT NULL CHECK (collector IN ('ldaps', 'kerberos')),
    status TEXT NOT NULL CHECK (status IN ('succeeded', 'partial', 'failed')),
    started_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ NOT NULL,
    domain_count BIGINT NOT NULL CHECK (domain_count BETWEEN 0 AND 64),
    entity_count BIGINT NOT NULL CHECK (entity_count BETWEEN 0 AND 3000),
    truncated BOOLEAN NOT NULL DEFAULT FALSE,
    inventory_applied BOOLEAN NOT NULL DEFAULT FALSE,
    issues JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (completed_at >= started_at),
    CHECK (jsonb_typeof(issues) = 'array'),
    CHECK (jsonb_array_length(issues) <= 100),
    CHECK (octet_length(issues::text) <= 65536)
);

CREATE INDEX idx_ad_collection_runs_target_received
    ON ad_collection_runs (network_id, collection_key, received_order DESC);
CREATE INDEX idx_ad_collection_runs_org
    ON ad_collection_runs (organization_id);
CREATE INDEX idx_ad_collection_runs_credential
    ON ad_collection_runs (credential_id);

CREATE TABLE ad_domains (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    network_id UUID NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    collection_key TEXT NOT NULL CHECK (char_length(collection_key) BETWEEN 1 AND 200),
    dns_name TEXT NOT NULL CHECK (char_length(dns_name) BETWEEN 1 AND 253),
    forest_dns_name TEXT NULL CHECK (char_length(forest_dns_name) BETWEEN 1 AND 253),
    netbios_name TEXT NULL CHECK (char_length(netbios_name) BETWEEN 1 AND 64),
    functional_level TEXT NULL CHECK (char_length(functional_level) BETWEEN 1 AND 100),
    last_collection_run_id UUID NOT NULL REFERENCES ad_collection_runs(id),
    observed_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (network_id, collection_key, dns_name)
);

CREATE INDEX idx_ad_domains_org ON ad_domains (organization_id);
CREATE INDEX idx_ad_domains_network ON ad_domains (network_id);

CREATE TABLE ad_entities (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    network_id UUID NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    domain_id UUID NOT NULL REFERENCES ad_domains(id) ON DELETE CASCADE,
    collection_run_id UUID NOT NULL REFERENCES ad_collection_runs(id),
    kind TEXT NOT NULL CHECK (kind IN (
        'domain_controller', 'site', 'subnet', 'trust', 'computer',
        'group', 'group_membership'
    )),
    external_id TEXT NOT NULL CHECK (char_length(external_id) BETWEEN 1 AND 512),
    name TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 256),
    dns_name TEXT NULL CHECK (char_length(dns_name) BETWEEN 1 AND 253),
    parent_external_id TEXT NULL CHECK (char_length(parent_external_id) BETWEEN 1 AND 512),
    related_external_id TEXT NULL CHECK (char_length(related_external_id) BETWEEN 1 AND 512),
    site_name TEXT NULL CHECK (char_length(site_name) BETWEEN 1 AND 256),
    operating_system TEXT NULL CHECK (char_length(operating_system) BETWEEN 1 AND 256),
    operating_system_version TEXT NULL CHECK (char_length(operating_system_version) BETWEEN 1 AND 128),
    network_prefix CIDR NULL,
    is_enabled BOOLEAN NULL,
    observed_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (domain_id, kind, external_id)
);

CREATE INDEX idx_ad_entities_org ON ad_entities (organization_id);
CREATE INDEX idx_ad_entities_network_kind ON ad_entities (network_id, kind);
CREATE INDEX idx_ad_entities_domain_kind ON ad_entities (domain_id, kind);
CREATE INDEX idx_ad_entities_collection_run ON ad_entities (collection_run_id);

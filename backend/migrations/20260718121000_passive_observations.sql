SET lock_timeout = '5s';
SET statement_timeout = '5s';

CREATE TABLE passive_observations (
    id UUID PRIMARY KEY,
    network_id UUID NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    daemon_id UUID NOT NULL REFERENCES daemons(id) ON DELETE CASCADE,
    source TEXT NOT NULL CHECK (source IN ('mdns', 'dhcp', 'kernel_neighbor', 'arp')),
    observation_key TEXT NOT NULL,
    confidence BIGINT NOT NULL CHECK (confidence BETWEEN 0 AND 100),
    correlation_kind TEXT NOT NULL,
    correlation_key TEXT NOT NULL,
    fact JSONB NOT NULL,
    observed_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (expires_at IS NULL OR expires_at >= observed_at),
    CHECK (pg_column_size(fact) <= 16384)
);

CREATE UNIQUE INDEX passive_observations_current_fact_idx
    ON passive_observations(network_id, daemon_id, source, observation_key);

CREATE INDEX passive_observations_network_time_idx
    ON passive_observations(network_id, observed_at DESC);
CREATE INDEX passive_observations_daemon_cap_idx
    ON passive_observations(network_id, daemon_id, observed_at DESC, id DESC);
CREATE INDEX passive_observations_expiry_idx
    ON passive_observations(network_id, daemon_id, expires_at)
    WHERE expires_at IS NOT NULL;
CREATE INDEX passive_observations_correlation_idx
    ON passive_observations(network_id, correlation_kind, correlation_key, observed_at DESC);

CREATE TABLE passive_correlations (
    network_id UUID NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    correlation_kind TEXT NOT NULL,
    correlation_key TEXT NOT NULL,
    first_seen_at TIMESTAMPTZ NOT NULL,
    last_seen_at TIMESTAMPTZ NOT NULL,
    observation_count BIGINT NOT NULL DEFAULT 1 CHECK (observation_count > 0),
    sources TEXT[] NOT NULL DEFAULT '{}',
    PRIMARY KEY (network_id, correlation_kind, correlation_key),
    CHECK (last_seen_at >= first_seen_at)
);

CREATE INDEX passive_correlations_last_seen_idx
    ON passive_correlations(network_id, last_seen_at DESC);

COMMENT ON TABLE passive_observations IS
    'Bounded structured passive facts only; raw frames and packet payloads are never stored.';
COMMENT ON COLUMN passive_observations.correlation_key IS
    'SHA-256 digest of a typed stable identifier; not a hostname/IP-only device merge.';
COMMENT ON COLUMN passive_observations.observation_key IS
    'SHA-256 digest of source plus structured fact, used to coalesce repeated current facts.';

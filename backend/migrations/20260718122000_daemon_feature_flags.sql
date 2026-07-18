-- Optional native daemon integrations cannot be negotiated by version alone:
-- two builds at the same version may have different Cargo features. Persist the
-- explicit feature advertisement from registration and status heartbeats.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE daemons
    ADD COLUMN feature_flags TEXT[] NOT NULL DEFAULT '{}';
